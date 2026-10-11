use ohmyloop::{
    audio::{Audio, write_wav},
    dsp::Controls,
    engine::{Engine, LoopRegion, RenderedFrame, render},
    playback::{Consumer, QueuedFrame, Shared},
    session::Session,
};
use std::{
    alloc::{GlobalAlloc, Layout, System},
    cell::Cell,
    sync::{Arc, atomic::Ordering},
};

thread_local! {
    static TRACK_ALLOCATIONS: Cell<bool> = const { Cell::new(false) };
    static ALLOCATIONS: Cell<usize> = const { Cell::new(0) };
}
struct CountingAllocator;
unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        TRACK_ALLOCATIONS.with(|tracking| {
            if tracking.get() {
                ALLOCATIONS.with(|n| n.set(n.get() + 1));
            }
        });
        unsafe { System.alloc(layout) }
    }
    unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
        unsafe { System.dealloc(pointer, layout) }
    }
    unsafe fn realloc(&self, pointer: *mut u8, layout: Layout, size: usize) -> *mut u8 {
        TRACK_ALLOCATIONS.with(|tracking| {
            if tracking.get() {
                ALLOCATIONS.with(|n| n.set(n.get() + 1));
            }
        });
        unsafe { System.realloc(pointer, layout, size) }
    }
}
#[global_allocator]
static ALLOCATOR: CountingAllocator = CountingAllocator;

fn frequency(frames: &[[f32; 2]], rate: u32) -> f64 {
    let frames = &frames[frames.len() / 4..frames.len() * 3 / 4];
    let crossings: Vec<f64> = frames
        .windows(2)
        .enumerate()
        .filter(|(_, p)| p[0][0] <= 0.0 && p[1][0] > 0.0)
        .map(|(n, p)| n as f64 - p[0][0] as f64 / (p[1][0] - p[0][0]) as f64)
        .collect();
    assert!(crossings.len() > 2);
    (crossings.len() - 1) as f64 * rate as f64 / (crossings.last().unwrap() - crossings[0])
}

#[test]
fn real_dsp_changes_speed_and_pitch_independently() {
    let audio = Arc::new(Audio::tone(48_000, 2.0, 440.0));
    // R3 is the supported playback engine. R2 is measured separately by probe.
    for finer in [true] {
        for (speed, semitones) in [
            (0.5, 0.0),
            (1.0, 7.0),
            (0.5, -5.0),
            (0.25, -12.0),
            (2.0, 12.0),
        ] {
            let controls = Controls { speed, semitones };
            let output = render(audio.clone(), controls, 0.0, None, None, finer).unwrap();
            let seconds = output.len() as f64 / audio.rate as f64;
            assert!(
                (seconds - 2.0 / speed).abs() < 0.05,
                "speed={speed} pitch={semitones} duration={seconds}"
            );
            let hz = frequency(&output, audio.rate);
            let cents = 1200.0 * (hz / (440.0 * controls.pitch_scale())).log2();
            assert!(
                cents.abs() < 10.0,
                "speed={speed} pitch={semitones} cents={cents}"
            );
            assert!(output.iter().flatten().all(|s| s.is_finite()));
        }
    }
}

#[test]
fn seek_and_loop_use_the_selected_audio_not_only_matching_metadata() {
    let mut audio = Audio::tone(48_000, 1.0, 220.0);
    audio.frames.extend(Audio::tone(48_000, 1.0, 880.0).frames);
    let audio = Arc::new(audio);
    let controls = Controls {
        speed: 0.75,
        semitones: 3.0,
    };
    for finer in [false, true] {
        let sought = render(audio.clone(), controls, 1.15, None, None, finer).unwrap();
        assert!((frequency(&sought, 48_000) - 880.0 * controls.pitch_scale()).abs() < 3.0);
        let looped = render(
            audio.clone(),
            controls,
            0.0,
            Some(LoopRegion {
                start: 1.2,
                end: 1.7,
                fade_ms: 5.0,
            }),
            Some(48_000 * 3),
            finer,
        )
        .unwrap();
        assert_eq!(looped.len(), 48_000 * 3);
        assert!((frequency(&looped, 48_000) - 880.0 * controls.pitch_scale()).abs() < 3.0);
        assert!(looped.iter().flatten().all(|x| x.is_finite()));
    }
}

#[test]
fn live_control_changes_reach_real_dsp() {
    let audio = Arc::new(Audio::tone(48_000, 10.0, 440.0));
    let region = Some(LoopRegion {
        start: 1.0,
        end: 2.0,
        fade_ms: 0.0,
    });
    let mut engine = Engine::new(audio, Controls::default(), 1.0, region, true).unwrap();
    let collect = |engine: &mut Engine| -> Vec<[f32; 2]> {
        let mut frames = Vec::new();
        while frames.len() < 48_000 * 2 {
            frames.extend(engine.next_block().into_iter().map(|f| f.samples));
        }
        frames
    };
    assert!((frequency(&collect(&mut engine), 48_000) - 440.0).abs() < 2.0);
    engine
        .set_controls(Controls {
            speed: 0.5,
            semitones: 12.0,
        })
        .unwrap();
    let shifted = collect(&mut engine);
    assert!((frequency(&shifted, 48_000) - 880.0).abs() < 3.0);
    assert!(engine.loop_wraps >= 3);
}

#[test]
fn paused_queue_is_preserved_and_old_seek_generation_is_never_played() {
    let shared = Arc::new(Shared::new(2048));
    let old = QueuedFrame {
        count_in: false,
        frame: RenderedFrame {
            samples: [0.8; 2],
            source_frame: 1.0,
        },
        epoch: 0,
    };
    for _ in 0..500 {
        shared
            .queue
            .push(old)
            .unwrap_or_else(|_| panic!("queue full"));
    }
    let mut consumer = Consumer::new(shared.clone(), 48_000);
    assert_eq!(consumer.next_frame(), [0.0; 2]);
    assert_eq!(shared.queue.len(), 500);
    shared.epoch.store(1, Ordering::Release);
    shared
        .queue
        .push(QueuedFrame {
            count_in: false,
            frame: RenderedFrame {
                samples: [-0.2; 2],
                source_frame: 50_000.0,
            },
            epoch: 1,
        })
        .unwrap_or_else(|_| panic!("queue full"));
    shared.playing.store(true, Ordering::Release);
    for _ in 0..241 {
        assert!(consumer.next_frame()[0] <= 0.0);
    }
    assert_eq!(shared.position(), 50_000.0);
    assert_eq!(shared.output_frames.load(Ordering::Relaxed), 1);
}

#[test]
fn callback_consumer_allocates_nothing_including_seek_pause_and_underflow() {
    let shared = Arc::new(Shared::new(2048));
    let mut consumer = Consumer::new(shared.clone(), 48_000);
    for n in 0..1024 {
        shared
            .queue
            .push(QueuedFrame {
                count_in: false,
                frame: RenderedFrame {
                    samples: [0.2; 2],
                    source_frame: n as f64,
                },
                epoch: 0,
            })
            .unwrap_or_else(|_| panic!("queue full"));
    }
    ALLOCATIONS.with(|n| n.set(0));
    TRACK_ALLOCATIONS.with(|n| n.set(true));
    shared.playing.store(true, Ordering::Release);
    for _ in 0..1200 {
        std::hint::black_box(consumer.next_frame());
    }
    shared.epoch.store(1, Ordering::Release);
    shared.playing.store(false, Ordering::Release);
    for _ in 0..300 {
        std::hint::black_box(consumer.next_frame());
    }
    TRACK_ALLOCATIONS.with(|n| n.set(false));
    assert_eq!(ALLOCATIONS.with(Cell::get), 0);
}

#[test]
fn source_markers_survive_session_serialization_and_reject_invalid_input() {
    let mut session = Session::new("test-song".into());
    session.markers.insert("chorus".into(), 1.25);
    session.controls = Controls {
        speed: 0.5,
        semitones: -3.5,
    };
    let json = serde_json::to_vec(&session).unwrap();
    let restored: Session = serde_json::from_slice(&json).unwrap();
    restored.validate(5.0).unwrap();
    assert_eq!(restored.markers["chorus"], 1.25);
    // Decimal boundaries that differ by 100 ms must not fail due to rounding.
    LoopRegion {
        start: 0.2,
        end: 0.3,
        fade_ms: 5.0,
    }
    .validate(5.0)
    .unwrap();
    assert!(
        Controls {
            speed: f64::NAN,
            semitones: 0.0
        }
        .validate()
        .is_err()
    );
    assert!(
        LoopRegion {
            start: 3.0,
            end: 2.0,
            fade_ms: 5.0
        }
        .validate(5.0)
        .is_err()
    );
    assert!(
        LoopRegion {
            start: 0.0,
            end: 0.001,
            fade_ms: 5.0
        }
        .validate(5.0)
        .is_err()
    );
}

#[test]
fn wav_decode_preserves_rate_length_and_stereo_channels() {
    let path = std::env::temp_dir().join(format!("ohmyloop-decode-{}.wav", std::process::id()));
    let mut original = Audio::tone(44_100, 0.5, 440.0);
    for frame in &mut original.frames {
        frame[1] *= -0.5;
    }
    write_wav(&path, original.rate, &original.frames).unwrap();
    let decoded = Audio::load(&path).unwrap();
    std::fs::remove_file(path).unwrap();
    assert_eq!(decoded.rate, original.rate);
    assert_eq!(decoded.frames, original.frames);
}

#[test]
fn finite_eof_drains_and_seek_exactly_to_end_terminates() {
    let audio = Arc::new(Audio::tone(48_000, 0.2, 440.0));
    assert!(
        render(
            audio.clone(),
            Controls::default(),
            audio.duration(),
            None,
            None,
            true
        )
        .unwrap()
        .is_empty()
    );
    let mut engine = Engine::new(audio, Controls::default(), 0.0, None, true).unwrap();
    let mut count = 0;
    for _ in 0..100 {
        let block = engine.next_block();
        if block.is_empty() {
            break;
        }
        count += block.len();
    }
    assert!(count > 8000 && count <= 9600, "received {count} frames");
    assert!(engine.next_block().is_empty());
}

#[test]
fn loop_fades_reduce_discontinuities_in_actual_rendered_audio() {
    // Deliberately discontinuous saw ramp, cut at non-zero endpoints.
    let rate = 48_000;
    let audio = Arc::new(Audio {
        rate,
        frames: (0..rate)
            .map(|n| {
                let value = (n % 9600) as f32 / 9600.0 - 0.5;
                [value, value]
            })
            .collect(),
    });
    let cut = |fade_ms| {
        render(
            audio.clone(),
            Controls::default(),
            0.0,
            Some(LoopRegion {
                start: 0.2,
                end: 0.4,
                fade_ms,
            }),
            Some(rate as usize * 2),
            true,
        )
        .unwrap()
    };
    let hard = cut(0.0);
    let faded = cut(5.0);
    let max_step = |frames: &[[f32; 2]]| {
        frames[rate as usize / 2..]
            .windows(2)
            .map(|pair| (pair[1][0] - pair[0][0]).abs())
            .fold(0.0_f32, f32::max)
    };
    let hard_step = max_step(&hard);
    let faded_step = max_step(&faded);
    assert!(
        hard_step > 0.5,
        "Fixture did not expose a hard loop seam: {hard_step}"
    );
    assert!(
        faded_step < hard_step * 0.1,
        "Fade ineffective: hard={hard_step}, faded={faded_step}"
    );
}

#[test]
fn repeated_loop_timing_does_not_drift_at_half_speed() {
    // One isolated pulse per source period; check spacing in DSP output.
    let rate = 48_000;
    let mut audio = Audio {
        rate,
        frames: vec![[0.0; 2]; rate as usize],
    };
    for n in 0..480 {
        // short, windowed 1000 Hz burst starting at 100 ms
        let envelope = (std::f32::consts::PI * n as f32 / 480.0).sin().powi(2);
        let value =
            0.4 * envelope * (std::f32::consts::TAU * 1000.0 * n as f32 / rate as f32).sin();
        audio.frames[4800 + n] = [value, value];
    }
    let output = render(
        Arc::new(audio),
        Controls {
            speed: 0.5,
            semitones: 0.0,
        },
        0.0,
        Some(LoopRegion {
            start: 0.0,
            end: 0.5,
            fade_ms: 5.0,
        }),
        Some(rate as usize * 6),
        true,
    )
    .unwrap();
    let mut peaks = Vec::new();
    for second in 1..5 {
        let begin = second * rate as usize;
        let window = &output[begin..begin + rate as usize];
        let index = window
            .iter()
            .enumerate()
            .max_by(|(_, a), (_, b)| a[0].abs().total_cmp(&b[0].abs()))
            .unwrap()
            .0;
        peaks.push(begin + index);
    }
    for pair in peaks.windows(2) {
        assert!(
            ((pair[1] - pair[0]) as f64 / rate as f64 - 1.0).abs() < 0.01,
            "Loop timing drift: {peaks:?}"
        );
    }
}
