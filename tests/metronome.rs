use ohmyloop::{
    audio::Audio,
    dsp::Controls,
    engine::{Engine, LoopRegion},
    metronome::{ClickMixer, ticks},
    timing::{Accent, Edit, Fraction, MeasureKind, MeasureMap, Meter},
};
use std::sync::Arc;

fn map(start: u64, end: u64, count: u32, meter: Meter) -> MeasureMap {
    MeasureMap::empty(1000, 20_000)
        .edit(Edit::Build {
            start,
            end,
            count,
            meter,
        })
        .unwrap()
        .map
}
#[test]
fn authored_pulses_cover_meter_grouping_pickups_partials_and_free_time() {
    let common = map(1000, 9000, 2, Meter::common());
    let events = common.pulse_events().unwrap();
    assert_eq!(
        events.iter().map(|e| e.frame).collect::<Vec<_>>(),
        [1000, 2000, 3000, 4000, 5000, 6000, 7000, 8000]
    );
    assert_eq!(
        events
            .iter()
            .filter(|e| e.accent == Accent::Measure)
            .count(),
        2
    );
    let compound = map(
        0,
        3000,
        1,
        Meter {
            numerator: 6,
            denominator: 8,
            pulse: Fraction::new(3, 2),
            grouping: vec![],
        },
    );
    assert_eq!(
        compound
            .pulse_events()
            .unwrap()
            .iter()
            .map(|e| e.frame)
            .collect::<Vec<_>>(),
        [0, 1500]
    );
    let odd = map(
        0,
        3500,
        1,
        Meter {
            numerator: 7,
            denominator: 8,
            pulse: Fraction::new(1, 2),
            grouping: vec![2, 2, 3],
        },
    );
    let events = odd.pulse_events().unwrap();
    assert_eq!(events.len(), 7); // Grouping must not discard the other pulses.
    assert_eq!(
        events
            .iter()
            .filter(|e| e.accent == Accent::Group)
            .map(|e| e.frame)
            .collect::<Vec<_>>(),
        [1000, 2000]
    );
    let pickup = common
        .edit(Edit::SetNotation {
            id: common.measures[0].id.clone(),
            meter: Meter::common(),
            kind: MeasureKind::Pickup,
            length: Fraction::new(1, 1),
            following: false,
        })
        .unwrap()
        .map;
    let partial = pickup
        .edit(Edit::SetNotation {
            id: pickup.measures[1].id.clone(),
            meter: Meter::common(),
            kind: MeasureKind::Incomplete,
            length: Fraction::new(3, 2),
            following: false,
        })
        .unwrap()
        .map;
    let events = partial.pulse_events().unwrap();
    assert_eq!(events[0].frame, 1000);
    assert_eq!(events[0].accent, Accent::Beat);
    assert_eq!(events.len(), 3);
    assert_eq!(events[1].frame, 5000);
    assert!(events[2].frame < 9000);
    let gap = partial
        .edit(Edit::Remove {
            id: partial.measures[0].id.clone(),
        })
        .unwrap()
        .map;
    assert!(
        gap.pulse_events()
            .unwrap()
            .iter()
            .all(|e| e.frame >= 5000 && e.frame < 9000)
    );
}
#[test]
fn arbitrary_anchors_change_pulse_timing_without_becoming_extra_clicks() {
    let common = map(0, 4000, 1, Meter::common());
    let anchored = common
        .edit(Edit::SetBeat {
            measure: common.measures[0].id.clone(),
            id: None,
            frame: 200,
            offset: Fraction::new(1, 2),
        })
        .unwrap()
        .map;
    let events = anchored.pulse_events().unwrap();
    assert_eq!(events.len(), 4);
    assert!(!events.iter().any(|e| e.frame == 200));
    assert_eq!(
        events[1].frame,
        anchored.frame_at(&anchored.measures[0], 1.0)
    );
    let output = ticks(&anchored, 44_100).unwrap();
    assert_eq!(
        output[1].source_frame,
        (events[1].frame as f64 * 44.1).round()
    );
    let mut mixer = ClickMixer::new(44_100);
    mixer.configure(true, 1.0, output.clone());
    assert!(
        (0..1000).any(|n| mixer.mix(output[1].source_frame + n as f64, [0.0; 2])[0].abs() > 0.01)
    );
    let tail = map(19_999, 20_000, 1, Meter::common());
    assert!(
        ticks(&tail, 100).unwrap().is_empty(),
        "Rounding must not create an EOF click"
    );
}
fn render_clicks(
    speed: f64,
    semitones: f64,
    start: f64,
    region: Option<LoopRegion>,
    limit: usize,
) -> Vec<f32> {
    let rate = 48_000;
    let audio = Arc::new(Audio {
        rate,
        frames: vec![[0.0; 2]; rate as usize * 2],
    });
    let authored = MeasureMap::empty(rate, rate as u64 * 2)
        .edit(Edit::Build {
            start: rate as u64 / 4,
            end: rate as u64 * 7 / 4,
            count: 2,
            meter: Meter {
                numerator: 3,
                ..Meter::common()
            },
        })
        .unwrap()
        .map;
    let mut mixer = ClickMixer::new(rate);
    mixer.configure(true, 1.0, ticks(&authored, rate).unwrap());
    mixer.reset(region);
    let mut engine =
        Engine::new(audio, Controls { speed, semitones }, start, region, true).unwrap();
    let mut result = vec![];
    while result.len() < limit {
        let block = engine.next_block();
        if block.is_empty() {
            break;
        }
        for frame in block {
            result.push(mixer.mix(frame.source_frame, frame.samples)[0]);
            if result.len() == limit {
                break;
            }
        }
    }
    assert!(result.iter().all(|s| s.is_finite() && s.abs() <= 1.0));
    result
}
fn onsets(samples: &[f32]) -> Vec<usize> {
    let mut result = vec![];
    let mut previous = None;
    for (index, sample) in samples.iter().enumerate() {
        if sample.abs() > 0.0001 {
            if previous.is_none_or(|last| index - last > 100) {
                result.push(index);
            }
            previous = Some(index);
        }
    }
    result
}
#[test]
fn actual_dsp_output_click_spacing_changes_with_speed_but_click_pitch_does_not() {
    for speed in [0.5, 1.0, 2.0] {
        let normal = render_clicks(speed, 0.0, 0.0, None, 48_000 * 5);
        let shifted = render_clicks(speed, -7.0, 0.0, None, 48_000 * 5);
        assert_eq!(
            normal, shifted,
            "Clicks are mixed after actual pitch processing"
        );
        let starts = onsets(&normal);
        assert_eq!(starts.len(), 6);
        for (i, start) in starts.into_iter().enumerate() {
            let expected = ((0.25 + i as f64 * 0.25) * 48_000.0 / speed).ceil() as usize;
            assert!(
                start.abs_diff(expected) <= 2,
                "speed={speed} onset={start} expected={expected}"
            );
        }
    }
}
#[test]
fn seeks_and_fractional_loop_wraps_do_not_replay_stale_or_exclusive_end_clicks() {
    let sought = render_clicks(1.0, 3.0, 1.1, None, 48_000);
    assert_eq!(onsets(&sought).len(), 2);
    assert!(onsets(&sought)[0].abs_diff(7200) <= 2);
    let region = LoopRegion {
        start: 0.250002,
        end: 0.8000208333333333,
        fade_ms: 5.0,
    };
    let looped = render_clicks(1.7, 5.0, 0.25, Some(region), 48_000 * 2);
    // Three mapped pulses inside this loop (0.25, 0.5, 0.75); a wrap must include 0.25
    // even when speed leaves a fractional overshoot past its exact source frame.
    let starts = onsets(&looped);
    assert!(starts.len() >= 18);
    let interval = 26_401.0 / 1.7;
    for iteration in 0..6 {
        let expected = (iteration as f64 * interval).ceil() as usize;
        assert!(
            starts.iter().any(|s| s.abs_diff(expected) <= 3),
            "Missing loop downbeat {iteration}: {starts:?}"
        );
    }
    let exclusive = render_clicks(
        1.0,
        0.0,
        0.25,
        Some(LoopRegion {
            end: 0.75,
            ..region
        }),
        48_000,
    );
    assert_eq!(onsets(&exclusive).len(), 4);
}
