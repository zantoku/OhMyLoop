//! Protection checks through prepared stems, real R3 DSP and production Consumer.
use anyhow::Result;
use ohmyloop::{
    audio::{Audio, write_wav},
    count_in::Settings,
    dsp::Controls,
    engine::{LoopRegion, MemorySource},
    metronome,
    playback::{Consumer, Player},
    stems::{BlockSource, MixState, PreparedComparison, Role, SourceFactory},
    timing::{Edit, MeasureMap, Meter},
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    fs,
    path::{Path, PathBuf},
    sync::{Arc, atomic::Ordering},
    thread,
    time::{Duration, Instant},
};
struct Fixture(PathBuf);
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
fn digest(path: &Path) -> String {
    format!("{:x}", Sha256::digest(fs::read(path).unwrap()))
}
fn prepared() -> (Fixture, Arc<PreparedComparison>) {
    let directory =
        Fixture(std::env::temp_dir().join(format!("ohmyloop-protection-{}", uuid::Uuid::new_v4())));
    fs::create_dir(&directory.0).unwrap();
    let reference = directory.0.join("reference.wav");
    let set = directory.0.join("set");
    fs::create_dir(&set).unwrap();
    let rate = 48_000;
    let frames = 96_000;
    let samples: Vec<[f32; 2]> = (0..frames)
        .map(|n| {
            let x = if n % 4800 < 24 {
                1.2
            } else {
                0.9 * (std::f32::consts::TAU * 440.0 * n as f32 / rate as f32).sin()
            };
            [x, x]
        })
        .collect();
    write_wav(&reference, rate, &samples).unwrap();
    write_wav(&set.join("prepared.wav"), rate, &samples).unwrap();
    let mut stems = serde_json::Map::new();
    let track: Vec<_> = samples.iter().map(|f| [f[0] / 6.0, f[1] / 6.0]).collect();
    for role in Role::ALL {
        let name = format!("{}.wav", role.name());
        write_wav(&set.join(&name), rate, &track).unwrap();
        stems.insert(role.name().into(),json!({"file":name,"channels":2,"rate":rate,"frames":frames,"sha256":digest(&set.join(name))}));
    }
    let identity = json!({"preparation":{"version":1,"channels":2,"source_sha256":digest(&reference),"source_rate":rate,"source_frames":frames,"source_span":{"start":0,"end_exclusive":frames},"prepared_rate":rate,"prepared_frames":frames,"prepared_sha256":digest(&set.join("prepared.wav"))}});
    let job = format!(
        "{:x}",
        Sha256::digest(serde_json::to_vec(&identity).unwrap())
    );
    let manifest: Value =
        json!({"version":1,"status":"complete","job_key":job,"identity":identity,"stems":stems});
    fs::write(
        set.join("manifest.json"),
        serde_json::to_vec(&manifest).unwrap(),
    )
    .unwrap();
    let prepared = Arc::new(PreparedComparison::prepare(&reference, &[set], rate, |_| {}).unwrap());
    (directory, prepared)
}
fn consume(player: &Player, consumer: &mut Consumer, count: usize) {
    let mut deadline = Instant::now() + Duration::from_secs(3);
    for _ in 0..count {
        while player.shared.queue.is_empty() {
            assert!(
                Instant::now() < deadline,
                "Worker error: {:?}",
                player.shared.worker_error.lock().unwrap()
            );
            thread::sleep(Duration::from_millis(1));
        }
        deadline = Instant::now() + Duration::from_secs(3);
        let frame = consumer.next_frame();
        assert!(frame.iter().all(|x| x.is_finite() && x.abs() <= 1.0));
    }
}
#[test]
fn prepared_near_bound_stem_mix_r3_music_and_maximum_clicks_do_not_clip() {
    let (_fixture, prepared) = prepared();
    let mut reader = prepared.reader().unwrap();
    reader.select_source(1).unwrap();
    let mut input = vec![[0.0; 2]; 96_000];
    reader.read_frames(0, &mut input).unwrap();
    let peak = input
        .iter()
        .flatten()
        .map(|x| x.abs())
        .fold(0.0f32, f32::max);
    assert!((peak - 0.70).abs() < 0.00001);
    let map = MeasureMap::empty(48_000, 96_000)
        .edit(Edit::Build {
            start: 0,
            end: 96_000,
            count: 2,
            meter: Meter::common(),
        })
        .unwrap()
        .map;
    for speed in [0.25, 0.5, 1.0, 2.0] {
        for semitones in [-12.0, 0.0, 12.0] {
            let player = Player::headless(prepared.clone(), true, 48_000).unwrap();
            player.set_mix(1, MixState::default()).unwrap();
            player.set_controls(Controls { speed, semitones }).unwrap();
            player
                .set_loop(Some(LoopRegion {
                    start: 0.2,
                    end: 1.2,
                    fade_ms: 5.0,
                }))
                .unwrap();
            player
                .set_metronome(true, 1.0, metronome::ticks(&map, 48_000).unwrap())
                .unwrap();
            player
                .set_count_in(
                    Settings {
                        enabled: true,
                        repeat: true,
                        volume: 1.0,
                    },
                    Some(map.clone()),
                )
                .unwrap();
            player.play().unwrap();
            let mut consumer = Consumer::new(player.shared.clone(), 48_000);
            // Enough output for initial count-in, music and another count-in at every speed.
            consume(&player, &mut consumer, (48_000.0 * 3.5 / speed) as usize);
            let peak = f64::from_bits(player.shared.pre_clamp_peak.load(Ordering::Relaxed));
            let clips = player.shared.clipped_samples.load(Ordering::Relaxed);
            println!(
                "speed={speed} pitch={semitones} actual_pre_clamp_peak={peak:.6} clipped_samples={clips}"
            );
            assert_eq!(
                clips, 0,
                "Ordinary near-bound music plus maximum clicks clipped: speed={speed} pitch={semitones} peak={peak}"
            );
            assert!(peak > 0.1);
        }
    }
}
struct UnsafeInput;
impl SourceFactory for UnsafeInput {
    fn rate(&self) -> u32 {
        48_000
    }
    fn len_frames(&self) -> u64 {
        96_000
    }
    fn open(&self, _: usize, _: MixState) -> Result<Box<dyn BlockSource>> {
        let mut audio = Audio::tone(48_000, 2.0, 440.0);
        for frame in &mut audio.frames {
            for x in frame {
                *x *= 10.0;
            }
        }
        Ok(Box::new(MemorySource(Arc::new(audio))))
    }
}
#[test]
fn deliberately_unattenuated_input_through_real_r3_exposes_clamp_activity() {
    // Deliberately bypass preparation's headroom policy to exercise the final
    // protection through actual R3 output, rather than injecting queued samples.
    let player = Player::headless(Arc::new(UnsafeInput), true, 48_000).unwrap();
    player
        .set_controls(Controls {
            speed: 0.5,
            semitones: 12.0,
        })
        .unwrap();
    player.play().unwrap();
    let mut consumer = Consumer::new(player.shared.clone(), 48_000);
    consume(&player, &mut consumer, 48_000);
    let peak = f64::from_bits(player.shared.pre_clamp_peak.load(Ordering::Relaxed));
    let clips = player.shared.clipped_samples.load(Ordering::Relaxed);
    println!("deliberate_actual_R3_pre_clamp_peak={peak:.6} clipped_samples={clips}");
    assert!(peak > 1.0);
    assert!(clips > 0);
}
