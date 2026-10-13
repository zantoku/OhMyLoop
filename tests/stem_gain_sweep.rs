//! Continuous rotary-style gain commands through file-backed stems, R3 and callback consumer.
use ohmyloop::{
    audio::{Audio, write_wav},
    playback::{Consumer, Player},
    stems::{MixState, PreparedComparison},
};
use sha2::{Digest, Sha256};
use std::{
    fs,
    sync::Arc,
    thread,
    time::{Duration, Instant},
};
fn hash(path: &std::path::Path) -> String {
    format!("{:x}", Sha256::digest(fs::read(path).unwrap()))
}
fn ready(player: &Player) {
    let deadline = Instant::now() + Duration::from_secs(2);
    while player.shared.queue.len() < 2400 {
        assert!(
            Instant::now() < deadline,
            "{:?}",
            player.shared.worker_error.lock().unwrap()
        );
        thread::sleep(Duration::from_millis(1));
    }
}
struct Fixture {
    root: std::path::PathBuf,
    factory: Arc<PreparedComparison>,
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}
fn fixture() -> Fixture {
    let root = std::env::temp_dir().join(format!("ohmyloop-gain-sweep-{}", uuid::Uuid::new_v4()));
    fs::create_dir_all(&root).unwrap();
    let root = root.canonicalize().unwrap();
    let set = root.join("set");
    fs::create_dir(&set).unwrap();
    let reference = root.join("reference.wav");
    let audio = Audio::tone(44100, 4., 1000.);
    write_wav(&reference, audio.rate, &audio.frames).unwrap();
    fs::copy(&reference, set.join("prepared.wav")).unwrap();
    fs::copy(&reference, set.join("bass.wav")).unwrap();
    let identity = serde_json::json!({"preparation":{"version":1,"source_sha256":hash(&reference),"source_rate":44100,"source_frames":audio.frames.len(),"source_span":{"start":0,"end_exclusive":audio.frames.len()},"prepared_rate":44100,"prepared_frames":audio.frames.len(),"prepared_sha256":hash(&set.join("prepared.wav")),"channels":2}});
    let manifest = serde_json::json!({"version":1,"status":"complete","job_key":format!("{:x}",Sha256::digest(serde_json::to_vec(&identity).unwrap())),"identity":identity,"stems":{"bass":{"file":"bass.wav","sha256":hash(&set.join("bass.wav")),"rate":44100,"frames":audio.frames.len(),"channels":2}}});
    fs::write(
        set.join("manifest.json"),
        serde_json::to_vec(&manifest).unwrap(),
    )
    .unwrap();
    let factory = Arc::new(PreparedComparison::prepare(&reference, &[set], 48000, |_| {}).unwrap());
    Fixture { root, factory }
}
fn exercise_gain_sweep(changes: bool, controls: ohmyloop::dsp::Controls, automatic: bool) {
    let fixture = fixture();
    let factory = fixture.factory.clone();
    let player = Player::headless(factory, true, 44100).unwrap();
    player.set_controls(controls).unwrap();
    let mut consumer = Consumer::new(player.shared.clone(), 48000);
    let mut mix = MixState::default();
    mix.tracks[0].gain = 0.95;
    player.set_mix(1, mix).unwrap();
    player.play().unwrap();
    ready(&player);
    for _ in 0..2400 {
        consumer.next_frame();
    }
    let epoch = player
        .shared
        .epoch
        .load(std::sync::atomic::Ordering::Relaxed);
    let start_position = player.shared.position();
    let mut samples = Vec::new();
    for step in 0..90 {
        let phase = step as f32 / 89.;
        mix.tracks[0].gain = if phase < 0.5 {
            if automatic {
                1. - phase * 1.3
            } else {
                0.95 - phase * 1.2
            }
        } else {
            0.35 + (phase - 0.5) * if automatic { 1.3 } else { 0.7 }
        };
        if changes {
            player
                .set_mix(
                    if automatic && (step == 0 || step == 89) {
                        0
                    } else {
                        1
                    },
                    mix,
                )
                .unwrap();
        }
        ready(&player);
        for _ in 0..768 {
            samples.push(consumer.next_frame()[0]);
        }
    }
    let rms: Vec<f32> = samples
        .as_chunks::<192>()
        .0
        .iter()
        .map(|block| (block.iter().map(|s| s * s).sum::<f32>() / 192.).sqrt())
        .collect();
    let dropouts = rms.iter().filter(|r| **r < 0.02).count();
    let min = rms.iter().copied().fold(f32::INFINITY, f32::min);
    println!(
        "gain sweep: windows={} dropout_windows={} min_rms={} epochs={} underflows={}",
        rms.len(),
        dropouts,
        min,
        player
            .shared
            .epoch
            .load(std::sync::atomic::Ordering::Relaxed),
        player
            .shared
            .underflow_frames
            .load(std::sync::atomic::Ordering::Relaxed)
    );
    assert_eq!(
        player
            .shared
            .epoch
            .load(std::sync::atomic::Ordering::Relaxed),
        epoch,
        "Live mix must retain DSP history"
    );
    assert!(player.shared.position() > start_position);
    assert_eq!(
        player
            .shared
            .clipped_samples
            .load(std::sync::atomic::Ordering::Relaxed),
        0
    );
    drop(consumer);
    drop(player);
    drop(fixture);
    assert_eq!(
        dropouts,
        0,
        "Repeated low-energy gaps while dragging: {} of {} windows; min RMS {}",
        dropouts,
        rms.len(),
        min
    );
}

#[test]
fn steady_file_backed_signal_has_no_audio_dropouts() {
    exercise_gain_sweep(false, ohmyloop::dsp::Controls::default(), false);
}
#[test]
fn continuous_file_backed_gain_sweep_has_no_repeated_audio_dropouts() {
    exercise_gain_sweep(true, ohmyloop::dsp::Controls::default(), false);
}

#[test]
fn continuous_gain_and_original_transitions_remain_clean_across_speed_and_pitch() {
    for speed in [0.25, 1., 2.] {
        for semitones in [-12., 0., 12.] {
            exercise_gain_sweep(true, ohmyloop::dsp::Controls { speed, semitones }, true);
        }
    }
}

#[test]
fn file_backed_live_mix_preserves_count_in_clicks_then_loop_transport() {
    use ohmyloop::{
        count_in::Settings,
        engine::LoopRegion,
        metronome,
        timing::{Edit, MeasureMap, Meter},
    };
    let fixture = fixture();
    let map = MeasureMap::empty(44100, 176400)
        .edit(Edit::Build {
            start: 0,
            end: 176400,
            count: 16,
            meter: Meter::common(),
        })
        .unwrap()
        .map;
    let capture = |updates: bool| {
        let player = Player::headless(fixture.factory.clone(), true, 44100).unwrap();
        player
            .set_loop(Some(LoopRegion {
                start: 0.,
                end: 0.25,
                fade_ms: 5.,
            }))
            .unwrap();
        player
            .set_metronome(true, 0.4, metronome::ticks(&map, 48000).unwrap())
            .unwrap();
        player
            .set_count_in(
                Settings {
                    enabled: true,
                    repeat: false,
                    volume: 0.4,
                },
                Some(map.clone()),
            )
            .unwrap();
        player.play().unwrap();
        let mut consumer = Consumer::new(player.shared.clone(), 48000);
        let mut mix = MixState::default();
        let mut clicks = Vec::new();
        ready(&player);
        for _ in 0..2400 {
            clicks.push(consumer.next_frame());
        }
        for step in 0..20 {
            if updates {
                mix.tracks[0].gain = 0.5 + step as f32 / 100.;
                player
                    .set_mix(if step % 2 == 0 { 1 } else { 0 }, mix)
                    .unwrap();
            }
            ready(&player);
            for _ in 0..480 {
                clicks.push(consumer.next_frame());
            }
        }
        assert!(
            player
                .shared
                .count_in
                .load(std::sync::atomic::Ordering::Acquire)
        );
        assert_eq!(player.shared.position(), 0.);
        // Once music is audible, repeated edits retain the loop and its source coordinates.
        for step in 0..70 {
            mix.tracks[0].gain = 0.5 + step as f32 / 200.;
            player.set_mix(1, mix).unwrap();
            ready(&player);
            for _ in 0..768 {
                assert!(consumer.next_frame().iter().all(|s| s.is_finite()));
            }
            assert!(player.shared.position() <= 12000.);
        }
        assert!(
            player
                .shared
                .loop_wraps
                .load(std::sync::atomic::Ordering::Relaxed)
                > 0
        );
        assert_eq!(
            player
                .shared
                .clipped_samples
                .load(std::sync::atomic::Ordering::Relaxed),
            0
        );
        clicks
    };
    let expected = capture(false);
    let actual = capture(true);
    let mismatch = expected.iter().zip(&actual).position(|(a, b)| a != b);
    assert!(
        mismatch.is_none(),
        "Live edits changed count-in click phase at {mismatch:?}"
    );
}
