use ohmyloop::{
    count_in::{CountIn, Plan, Settings},
    timing::{Accent, Edit, Fraction, MeasureMap, Meter},
};

fn map(meter: Meter) -> MeasureMap {
    MeasureMap::empty(48_000, 480_000)
        .edit(Edit::Build {
            start: 48_000,
            end: 240_000,
            count: 2,
            meter,
        })
        .unwrap()
        .map
}

#[test]
fn local_meter_tempo_anchor_and_output_rate_define_the_count_in() {
    let authored = map(Meter::common());
    let plan = Plan::at(&authored, 1.0, 44_100).unwrap();
    assert!((plan.length - 88_200.0).abs() < 1e-6);
    assert_eq!(
        plan.ticks
            .iter()
            .map(|t| t.source_frame.round() as u64)
            .collect::<Vec<_>>(),
        [0, 22_050, 44_100, 66_150]
    );
    assert_eq!(plan.ticks[0].accent, Accent::Measure);
    let id = authored.measures[0].id.clone();
    let anchored = authored
        .edit(Edit::SetBeat {
            measure: id,
            id: None,
            offset: Fraction::new(1, 1),
            frame: 60_000,
        })
        .unwrap()
        .map;
    assert!((Plan::at(&anchored, 1.0, 48_000).unwrap().length - 48_000.0).abs() < 1e-6);
    assert!((Plan::at(&anchored, 1.25, 48_000).unwrap().length - 112_000.0).abs() < 1e-6);
    let compound = map(Meter {
        numerator: 6,
        denominator: 8,
        pulse: Fraction::new(3, 2),
        grouping: vec![3, 3],
    });
    let plan = Plan::at(&compound, 1.0, 48_000).unwrap();
    assert_eq!(plan.ticks.len(), 2);
    assert_eq!(plan.ticks[1].accent, Accent::Group);
    let odd = map(Meter {
        numerator: 7,
        denominator: 8,
        pulse: Fraction::new(1, 2),
        grouping: vec![2, 2, 3],
    });
    let plan = Plan::at(&odd, 1.0, 48_000).unwrap();
    assert_eq!(plan.ticks.len(), 7);
    assert_eq!(plan.ticks[2].accent, Accent::Group);
    assert_eq!(plan.ticks[4].accent, Accent::Group);
}

#[test]
fn pickups_mid_pulse_starts_and_free_time_have_a_defined_phase() {
    let authored = map(Meter::common());
    let plan = Plan::at(&authored, 1.125, 48_000).unwrap();
    assert!((plan.ticks[0].source_frame - 18_000.0).abs() < 1e-6);
    assert!((plan.ticks[3].source_frame - 90_000.0).abs() < 1e-6);
    // Leading free time uses the first mapped downbeat; an internal gap uses
    // the nearest mapped frame, rather than a fabricated global BPM.
    assert_eq!(
        Plan::at(&authored, 0.0, 48_000).unwrap().ticks[0].source_frame,
        0.0
    );
    assert!(Plan::at(&authored, 9.0, 48_000).unwrap().length > 0.0);
    let id = authored.measures[0].id.clone();
    let pickup = authored
        .edit(Edit::SetNotation {
            id,
            meter: Meter::common(),
            kind: ohmyloop::timing::MeasureKind::Pickup,
            length: Fraction::new(1, 1),
            following: false,
        })
        .unwrap()
        .map;
    let plan = Plan::at(&pickup, 1.0, 48_000).unwrap();
    assert_eq!(plan.ticks.len(), 4);
    assert_eq!(plan.ticks[0].accent, Accent::Beat);
    assert_eq!(plan.ticks[1].accent, Accent::Measure);
    assert!((plan.length - 384_000.0).abs() < 1e-6);
}

#[test]
fn rendered_click_duration_and_spacing_follow_speed_and_volume() {
    let authored = map(Meter::common());
    for speed in [0.5, 1.0, 2.0] {
        let mut count = CountIn::new(
            Plan::at(&authored, 1.0, 48_000).unwrap(),
            48_000,
            1.0,
            48_000.0,
        );
        let mut samples = vec![];
        while let Some(frame) = count.next(speed) {
            samples.push(frame[0]);
        }
        assert_eq!(samples.len(), (96_000.0 / speed) as usize);
        for beat in 0..4 {
            let start = (beat as f64 * 24_000.0 / speed) as usize;
            assert!(samples[start..start + 1_000].iter().any(|s| s.abs() > 0.01));
            if beat > 0 {
                assert_eq!(
                    &samples[start..start + 1_000],
                    &samples[(24_000.0 / speed) as usize..(24_000.0 / speed) as usize + 1_000]
                );
            }
        }
    }
    let mut muted = CountIn::new(
        Plan::at(&authored, 1.0, 48_000).unwrap(),
        48_000,
        0.0,
        48_000.0,
    );
    while let Some(frame) = muted.next(2.0) {
        assert_eq!(frame, [0.0; 2]);
    }
    assert!(Plan::at(&MeasureMap::empty(48_000, 480_000), 0.0, 48_000).is_err());
    assert!(
        Settings {
            volume: f32::NAN,
            ..Settings::default()
        }
        .validate()
        .is_err()
    );
}

#[test]
#[ignore = "Requires a native audio output device; plays generated clicks and tone"]
fn native_count_in_smoke() {
    use ohmyloop::{audio::Audio, engine::LoopRegion, playback::Player};
    use std::{
        sync::atomic::Ordering,
        thread,
        time::{Duration, Instant},
    };
    let player = Player::open(Audio::tone(48_000, 3.0, 440.0), true).unwrap();
    let authored = MeasureMap::empty(48_000, 144_000)
        .edit(Edit::Build {
            start: 0,
            end: 144_000,
            count: 3,
            meter: Meter::common(),
        })
        .unwrap()
        .map;
    player
        .set_loop(Some(LoopRegion {
            start: 1.0,
            end: 1.5,
            fade_ms: 5.0,
        }))
        .unwrap();
    player
        .set_count_in(
            Settings {
                enabled: true,
                repeat: true,
                volume: 0.5,
            },
            Some(authored),
        )
        .unwrap();
    player.play().unwrap();
    let deadline = Instant::now() + Duration::from_secs(5);
    let mut stages = vec![];
    while Instant::now() < deadline {
        let counting = player.shared.count_in.load(Ordering::Relaxed);
        if stages.last() != Some(&counting) {
            stages.push(counting);
        }
        if stages.ends_with(&[true, false, true]) {
            break;
        }
        assert_eq!(player.shared.device_errors.load(Ordering::Relaxed), 0);
        assert!(player.shared.worker_error.lock().unwrap().is_none());
        thread::sleep(Duration::from_millis(10));
    }
    player.pause();
    assert!(
        stages.windows(3).any(|s| s == [true, false, true]),
        "Expected count-in, song, repetition count-in: {stages:?}"
    );
    assert_eq!(player.shared.underflow_frames.load(Ordering::Relaxed), 0);
    assert!(player.shared.output_frames.load(Ordering::Relaxed) > 0);
    println!("Native count-in stages {stages:?}: {}", player.status());
}
