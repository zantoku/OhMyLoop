use anyhow::Result;
use ohmyloop::{
    audio::Audio,
    dsp::Controls,
    engine::{Engine, LoopRegion, MemorySource},
    playback::{Consumer, Player},
    stems::{BlockSource, MixState, SourceFactory},
};
use std::{
    sync::{Arc, atomic::Ordering},
    thread,
    time::{Duration, Instant},
};

struct Fixture;
impl SourceFactory for Fixture {
    fn rate(&self) -> u32 {
        48_000
    }
    fn len_frames(&self) -> u64 {
        144_000
    }
    fn open(&self, selected: usize, mix: MixState) -> Result<Box<dyn BlockSource>> {
        anyhow::ensure!(selected <= 2, "Bad source");
        let mut audio = Audio::tone(48_000, 3.0, if selected == 2 { 880.0 } else { 440.0 });
        if selected != 0 {
            let anysolo = mix.tracks.iter().any(|t| t.solo);
            let t = mix.tracks[0];
            let gain = if t.mute || (anysolo && !t.solo) {
                0.0
            } else {
                t.gain
            };
            for frame in &mut audio.frames {
                frame.iter_mut().for_each(|v| *v *= gain);
            }
        }
        Ok(Box::new(MemorySource(Arc::new(audio))))
    }
}
fn wait(player: &Player) {
    let end = Instant::now() + Duration::from_secs(3);
    while player.shared.queue.len() < 2400 {
        assert!(
            Instant::now() < end,
            "Worker error: {:?}",
            player.shared.worker_error.lock().unwrap()
        );
        thread::sleep(Duration::from_millis(1));
    }
}
fn read(player: &Player, consumer: &mut Consumer, count: usize) -> Vec<[f32; 2]> {
    let mut result = Vec::with_capacity(count);
    for _ in 0..count {
        if player.shared.queue.is_empty() {
            wait(player);
        }
        result.push(consumer.next_frame());
    }
    result
}
fn hz(frames: &[[f32; 2]]) -> f64 {
    let data = &frames[frames.len() / 4..frames.len() * 3 / 4];
    let crosses: Vec<_> = data
        .windows(2)
        .enumerate()
        .filter(|(_, w)| w[0][0] <= 0.0 && w[1][0] > 0.0)
        .map(|(i, w)| i as f64 - w[0][0] as f64 / (w[1][0] - w[0][0]) as f64)
        .collect();
    (crosses.len() - 1) as f64 * 48_000.0 / (crosses.last().unwrap() - crosses[0])
}
#[test]
fn block_source_real_r3_pitch_duration_and_seek_loop() {
    for speed in [0.25, 0.5, 1.0, 1.5, 2.0] {
        for semitones in [-12.0, 0.0, 12.0] {
            let mut engine = Engine::new_source(
                Fixture.open(0, MixState::default()).unwrap(),
                Controls { speed, semitones },
                0.0,
                None,
                true,
            )
            .unwrap();
            assert_eq!(engine.startup().2, 3);
            let mut samples = Vec::new();
            loop {
                let block = engine.try_next_block().unwrap();
                if block.is_empty() {
                    break;
                }
                samples.extend(block.into_iter().map(|f| f.samples));
            }
            assert!((samples.len() as f64 / 48_000.0 - 3.0 / speed).abs() < 0.05);
            assert!(
                (1200.0 * (hz(&samples) / (440.0 * 2_f64.powf(semitones / 12.0))).log2()).abs()
                    < 10.0
            );
        }
    }
    let region = LoopRegion {
        start: 1.0,
        end: 1.5,
        fade_ms: 5.0,
    };
    let mut engine = Engine::new_source(
        Fixture.open(2, MixState::default()).unwrap(),
        Controls::default(),
        1.2,
        Some(region),
        true,
    )
    .unwrap();
    let mut samples = Vec::new();
    while samples.len() < 96_000 {
        let block = engine.try_next_block().unwrap();
        assert!(
            block
                .iter()
                .all(|f| (48_000.0..72_000.0).contains(&f.source_frame))
        );
        samples.extend(block.into_iter().map(|f| f.samples));
    }
    assert!((hz(&samples) - 880.0).abs() < 3.0);
}
#[test]
fn live_mix_source_paused_controls_and_original_bypass() {
    let player = Player::headless(Arc::new(Fixture), true, 48_000).unwrap();
    let mut consumer = Consumer::new(player.shared.clone(), 48_000);
    player.play().unwrap();
    read(&player, &mut consumer, 12_000);
    let before = player.shared.position();
    let mut mix = MixState::default();
    mix.tracks[0].mute = true;
    player.set_mix(1, mix).unwrap();
    wait(&player);
    assert!((player.shared.position() - before).abs() < 1.0);
    let silence = read(&player, &mut consumer, 12_000);
    assert!(
        silence[1000..]
            .iter()
            .all(|f| f.iter().all(|x| x.abs() < 1e-7))
    );
    player.set_mix(0, mix).unwrap();
    let original = read(&player, &mut consumer, 12_000);
    assert!((hz(&original) - 440.0).abs() < 3.0);
    mix.tracks[0].mute = false;
    mix.tracks[0].gain = 0.5;
    mix.tracks[0].solo = true;
    player.set_mix(2, mix).unwrap();
    let selected = read(&player, &mut consumer, 12_000);
    assert!((hz(&selected) - 880.0).abs() < 3.0);
    player.pause();
    let paused = player.shared.position();
    mix.tracks[0].mute = true;
    player.set_mix(1, mix).unwrap();
    player
        .set_controls(Controls {
            speed: 0.25,
            semitones: 12.0,
        })
        .unwrap();
    for _ in 0..1000 {
        consumer.next_frame();
    }
    assert_eq!(paused, player.shared.position());
    assert!(!player.shared.playing.load(Ordering::Acquire));
    player.play().unwrap();
    let silence = read(&player, &mut consumer, 12_000);
    assert!(
        silence[1000..]
            .iter()
            .all(|f| f.iter().all(|x| x.abs() < 1e-7))
    );
}
struct Failing;
impl BlockSource for Failing {
    fn rate(&self) -> u32 {
        48_000
    }
    fn len_frames(&self) -> u64 {
        48_000
    }
    fn read_frames(&mut self, _: u64, _: &mut [[f32; 2]]) -> Result<usize> {
        anyhow::bail!("Injected read failure")
    }
}
struct FailingFactory;
impl SourceFactory for FailingFactory {
    fn rate(&self) -> u32 {
        48_000
    }
    fn len_frames(&self) -> u64 {
        48_000
    }
    fn open(&self, _: usize, _: MixState) -> Result<Box<dyn BlockSource>> {
        Ok(Box::new(Failing))
    }
}
#[test]
fn read_failure_stops_worker_and_reports_error() {
    let player = Player::headless(Arc::new(FailingFactory), true, 48_000).unwrap();
    let end = Instant::now() + Duration::from_secs(3);
    while player.shared.worker_error.lock().unwrap().is_none() {
        assert!(Instant::now() < end);
        thread::sleep(Duration::from_millis(1));
    }
    assert!(!player.shared.playing.load(Ordering::Acquire));
    assert!(
        player
            .shared
            .worker_error
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .contains("Injected read failure")
    );
    assert!(player.play().is_err());
}

#[test]
fn variable_tempo_metronome_and_count_in_survive_mix_changes() {
    use ohmyloop::{
        count_in::Settings,
        metronome,
        timing::{Edit, MeasureMap, Meter},
    };
    let mut map = MeasureMap::empty(48_000, 144_000)
        .edit(Edit::Build {
            start: 0,
            end: 144_000,
            count: 6,
            meter: Meter::common(),
        })
        .unwrap()
        .map;
    let boundary = map.boundaries[1].id.clone();
    map = map
        .edit(Edit::MoveBoundary {
            id: boundary,
            frame: 18_000,
        })
        .unwrap()
        .map;
    let player = Player::headless(Arc::new(Fixture), true, 48_000).unwrap();
    player
        .set_metronome(true, 0.5, metronome::ticks(&map, 48_000).unwrap())
        .unwrap();
    player
        .set_count_in(
            Settings {
                enabled: true,
                repeat: true,
                volume: 0.5,
            },
            Some(map),
        )
        .unwrap();
    player
        .set_loop(Some(LoopRegion {
            start: 0.0,
            end: 0.375,
            fade_ms: 5.0,
        }))
        .unwrap();
    player.play().unwrap();
    let mut consumer = Consumer::new(player.shared.clone(), 48_000);
    read(&player, &mut consumer, 2400);
    assert!(player.shared.count_in.load(Ordering::Acquire));
    let before = player.shared.position();
    player.set_mix(2, MixState::default()).unwrap();
    assert_eq!(before, player.shared.position());
    let frames = read(&player, &mut consumer, 96_000);
    assert!(frames.iter().flatten().all(|x| x.is_finite()));
    assert_eq!(player.shared.clipped_samples.load(Ordering::Relaxed), 0);
    assert!(player.shared.loop_wraps.load(Ordering::Relaxed) > 0);
}

#[test]
fn consumer_exposes_pre_clamp_peaks_and_deliberate_overshoot_counts() {
    use ohmyloop::{
        engine::RenderedFrame,
        playback::{QueuedFrame, Shared},
    };
    let shared = Arc::new(Shared::new(1024));
    shared.playing.store(true, Ordering::Release);
    for _ in 0..1024 {
        shared
            .queue
            .push(QueuedFrame {
                frame: RenderedFrame {
                    samples: [2.0, -2.0],
                    source_frame: 0.0,
                },
                epoch: 0,
                count_in: false,
            })
            .ok()
            .unwrap();
    }
    let mut consumer = Consumer::new(shared.clone(), 48_000);
    for _ in 0..1024 {
        assert!(consumer.next_frame().iter().all(|x| x.abs() <= 1.0));
    }
    assert!(f64::from_bits(shared.pre_clamp_peak.load(Ordering::Relaxed)) >= 2.0);
    assert!(shared.clipped_samples.load(Ordering::Relaxed) > 0);
}

#[test]
fn fractional_rate_count_in_validates_canonical_reference_geometry() {
    use ohmyloop::{
        count_in::Settings,
        stems::PreparedComparison,
        timing::{Edit, MeasureMap, Meter},
    };
    let path = std::env::temp_dir().join(format!("ohmyloop-map-{}.wav", uuid::Uuid::new_v4()));
    let audio = Audio::tone(44_100, 44_101.0 / 44_100.0, 440.0);
    ohmyloop::audio::write_wav(&path, audio.rate, &audio.frames).unwrap();
    let prepared = Arc::new(PreparedComparison::prepare(&path, &[], 48_000, |_| {}).unwrap());
    std::fs::remove_file(path).unwrap();
    assert_eq!(prepared.reference_frames, 44_101);
    assert_eq!(prepared.frames, 48_001);
    let map = MeasureMap::empty(44_100, 44_101)
        .edit(Edit::Build {
            start: 0,
            end: 44_101,
            count: 2,
            meter: Meter::common(),
        })
        .unwrap()
        .map;
    let player = Player::headless(prepared, true, 44_100).unwrap();
    player
        .set_count_in(
            Settings {
                enabled: true,
                repeat: false,
                volume: 1.0,
            },
            Some(map.clone()),
        )
        .unwrap();
    let mut wrong = map.clone();
    wrong.frames += 1;
    assert!(
        player
            .set_count_in(Settings::default(), Some(wrong))
            .is_err()
    );
    player.play().unwrap();
    let mut consumer = Consumer::new(player.shared.clone(), 48_000);
    read(&player, &mut consumer, 8000);
    assert!(player.shared.count_in.load(Ordering::Acquire));
    assert_eq!(player.shared.position(), 0.0);
    player.set_mix(0, MixState::default()).unwrap();
    read(&player, &mut consumer, 20_000);
    assert!(player.shared.count_in.load(Ordering::Acquire));
    read(&player, &mut consumer, 4000);
    assert!(!player.shared.count_in.load(Ordering::Acquire));
    assert!((0.0..2000.0).contains(&player.shared.position()));
}
