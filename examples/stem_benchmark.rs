// SPDX-License-Identifier: GPL-3.0-or-later
//! Fresh-process RSS and actual production-worker/consumer change measurements.
#[path = "stem_player/fixtures.rs"]
mod fixtures;
#[path = "stem_player/memory.rs"]
mod memory;
use anyhow::{Result, ensure};
use ohmyloop::{
    dsp::Controls,
    playback::{Consumer, Player},
    stems::{MixState, PreparedComparison},
};
use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
    thread,
    time::{Duration, Instant},
};
const RATE: u32 = 48_000;
fn main() -> Result<()> {
    let arg = std::env::args().nth(1).unwrap_or_else(|| "15".into());
    if arg == "--help" {
        println!(
            "cargo run --locked --release --example stem_benchmark -- SECONDS [--hardware]\n--hardware plays an eight-second generated-stem device smoke.\nRun 15 and 300 in separate fresh processes. Generated reference44.1k/two six-role48k sets. RSS uses ps, supported on macOS/Linux. Wall-clock paced actual production worker/Consumer; no physical output device."
        );
        return Ok(());
    }
    let seconds: u32 = arg.parse()?;
    ensure!(seconds >= 10, "Use at least 10 seconds");
    let fixture = fixtures::Fixture::new(seconds)?;
    let sampler = memory::MemorySampler::start()?;
    let prepared = Arc::new(PreparedComparison::prepare(
        &fixture.reference,
        &fixture.sets,
        RATE,
        |_| {},
    )?);
    println!(
        "duration_s={seconds} baseline_rss_bytes={} preparation_peak_rss_bytes={} preparation_s={:.3} temporary_pcm_bytes={} shared_attenuation={:.6}",
        sampler.baseline,
        sampler.peak(),
        prepared.elapsed.as_secs_f64(),
        prepared.temp_bytes,
        prepared.shared_attenuation
    );
    let reference_rate = prepared.reference_rate;
    if std::env::args().any(|arg| arg == "--hardware") {
        ensure!(
            Player::output_rate()? == RATE,
            "Hardware smoke expects 48 kHz device"
        );
        let player = Player::open_bounded(prepared, true, reference_rate)?;
        let mut mix = MixState::default();
        player.set_mix(1, mix)?;
        player.play()?;
        for index in 0..4 {
            thread::sleep(Duration::from_secs(2));
            match index {
                0 => player.set_mix(2, mix)?,
                1 => {
                    mix.tracks[0].gain = 0.5;
                    player.set_mix(2, mix)?;
                }
                2 => player.set_controls(Controls {
                    speed: 0.25,
                    semitones: -12.0,
                })?,
                _ => player.seek(1.0)?,
            }
        }
        player.pause();
        if let Some(error) = player.shared.worker_error.lock().unwrap().as_ref() {
            anyhow::bail!("Hardware worker: {error}");
        }
        ensure!(
            player.shared.device_errors.load(Ordering::Relaxed) == 0,
            "Device error"
        );
        ensure!(
            player.shared.callbacks.load(Ordering::Relaxed) > 0,
            "No hardware callback frames"
        );
        println!(
            "Hardware smoke device={} rate={} pre_clamp_peak={} clipped_samples={} {}. Not a subjective listening verdict.",
            player.device_name,
            player.rate,
            f64::from_bits(player.shared.pre_clamp_peak.load(Ordering::Relaxed)),
            player.shared.clipped_samples.load(Ordering::Relaxed),
            player.status()
        );
        return Ok(());
    }
    let player = Player::headless(prepared, true, reference_rate)?;
    let mut mix = MixState::default();
    player.set_mix(1, mix)?;
    player.play()?;
    let observer = Observer::start(Consumer::new(player.shared.clone(), RATE));

    thread::sleep(Duration::from_millis(600));
    let mut worst_normal = Duration::ZERO;
    let mut worst_extreme = Duration::ZERO;
    for controls in [
        Controls::default(),
        Controls {
            speed: 0.5,
            semitones: -5.0,
        },
        Controls {
            speed: 0.25,
            semitones: -12.0,
        },
        Controls {
            speed: 0.25,
            semitones: 12.0,
        },
        Controls {
            speed: 2.0,
            semitones: -12.0,
        },
        Controls {
            speed: 2.0,
            semitones: 12.0,
        },
    ] {
        player.pause();
        player.seek(1.0)?;
        player.set_controls(controls)?;
        mix = MixState::default();
        player.set_mix(1, mix)?;
        player.play()?;

        thread::sleep(Duration::from_millis(600));
        let steady_start = player.shared.underflow_frames.load(Ordering::Relaxed);
        thread::sleep(Duration::from_millis(200));
        println!(
            "steady_200ms_underrun_events={}",
            player.shared.underflow_frames.load(Ordering::Relaxed) - steady_start
        );
        let start_underflows = player.shared.underflow_frames.load(Ordering::Relaxed);
        for repetition in 0..3 {
            // Each measurement observes changed audio, not command acknowledgement.
            let start = Instant::now();
            match repetition {
                0 => mix.tracks[0].gain = 0.0,
                1 => mix.tracks[0].mute = true,
                _ => mix.tracks[1].solo = true,
            }
            player.set_mix(1, mix)?;
            let silence = detect(&observer, |rms| rms < 0.0001, start)?;
            mix = MixState::default();
            let start = Instant::now();
            player.set_mix(1, mix)?;
            let regain = detect(&observer, |rms| rms > 0.025, start)?;
            let start = Instant::now();
            observer.max_silent_blocks.store(0, Ordering::Release);
            player.set_mix(2, mix)?;
            let switch = detect(&observer, |rms| (0.008..0.04).contains(&rms), start)?;
            println!(
                "source_switch_silent_gap_ms={}",
                observer.max_silent_blocks.load(Ordering::Acquire) * 10
            );
            let start = Instant::now();
            player.set_mix(1, mix)?;
            let switch_back = detect(&observer, |rms| rms > 0.045, start)?;
            let worst = [silence, regain, switch, switch_back]
                .into_iter()
                .max()
                .unwrap();
            if controls.speed == 1.0 && controls.semitones == 0.0 {
                worst_normal = worst_normal.max(worst);
            } else {
                worst_extreme = worst_extreme.max(worst);
            }
            println!(
                "speed={} pitch={} repeat={} silence_ms={:.3} regain_ms={:.3} switch_ms={:.3} switch_back_ms={:.3}",
                controls.speed,
                controls.semitones,
                repetition,
                silence.as_secs_f64() * 1000.0,
                regain.as_secs_f64() * 1000.0,
                switch.as_secs_f64() * 1000.0,
                switch_back.as_secs_f64() * 1000.0
            );
        }
        println!(
            "control_underrun_events={}",
            player.shared.underflow_frames.load(Ordering::Relaxed) - start_underflows
        );
        // Burst commands deliberately outpace consumer: only final state must emerge.
        let start = Instant::now();
        for index in 0..8 {
            mix.tracks[0].mute = index % 2 == 0;
            player.set_mix(if index % 2 == 0 { 2 } else { 1 }, mix)?;
        }
        let rapid = detect(&observer, |rms| rms > 0.045, start)?;
        if controls.speed == 1.0 && controls.semitones == 0.0 {
            worst_normal = worst_normal.max(rapid);
        } else {
            worst_extreme = worst_extreme.max(rapid);
        }
        println!(
            "rapid_eight_commands_final_audio_ms={:.3}",
            rapid.as_secs_f64() * 1000.0
        );
    }
    player.pause();
    if let Some(error) = player.shared.worker_error.lock().unwrap().as_ref() {
        anyhow::bail!("Worker error: {error}");
    }
    let peak = sampler.peak();
    let incremental = peak.saturating_sub(sampler.baseline);
    println!(
        "peak_rss_bytes={peak} incremental_peak_bytes={incremental} pre_clamp_peak={} clipped_samples={} total_underrun_events={} worst_normal_ms={:.3} worst_other_ms={:.3}",
        f64::from_bits(player.shared.pre_clamp_peak.load(Ordering::Relaxed)),
        player.shared.clipped_samples.load(Ordering::Relaxed),
        player.shared.underflow_frames.load(Ordering::Relaxed),
        worst_normal.as_secs_f64() * 1000.0,
        worst_extreme.as_secs_f64() * 1000.0
    );
    ensure!(
        incremental <= 256 * 1024 * 1024,
        "Incremental memory target failed"
    );
    ensure!(
        worst_normal <= Duration::from_millis(250) && worst_extreme <= Duration::from_millis(500),
        "Changed-output latency target failed; architecture review required"
    );
    ensure!(
        player.shared.clipped_samples.load(Ordering::Relaxed) == 0,
        "Ordinary fixture clipped"
    );
    Ok(())
}
struct Observer {
    rms: Arc<AtomicU64>,
    blocks: Arc<AtomicU64>,
    stop: Arc<AtomicBool>,
    max_silent_blocks: Arc<AtomicU64>,
    worker: Option<thread::JoinHandle<()>>,
}
impl Observer {
    fn start(mut consumer: Consumer) -> Self {
        let rms = Arc::new(AtomicU64::new(0));
        let blocks = Arc::new(AtomicU64::new(0));
        let stop = Arc::new(AtomicBool::new(false));
        let max_silent_blocks = Arc::new(AtomicU64::new(0));
        let gaps = max_silent_blocks.clone();
        let (r, b, s) = (rms.clone(), blocks.clone(), stop.clone());
        let worker = thread::spawn(move || {
            let start = Instant::now();
            let mut frames = 0u64;
            let mut silent_blocks = 0;
            while !s.load(Ordering::Acquire) {
                let mut energy = 0.0;
                for _ in 0..480 {
                    let frame = consumer.next_frame();
                    energy += f64::from(frame[0]).powi(2);
                }
                let rms = (energy / 480.0).sqrt();
                silent_blocks = if rms < 0.0001 { silent_blocks + 1 } else { 0 };
                gaps.fetch_max(silent_blocks, Ordering::Relaxed);
                r.store(rms.to_bits(), Ordering::Release);
                b.fetch_add(1, Ordering::Release);
                frames += 480;
                let deadline = start + Duration::from_secs_f64(frames as f64 / f64::from(RATE));
                if let Some(wait) = deadline.checked_duration_since(Instant::now()) {
                    thread::sleep(wait);
                }
            }
        });
        Self {
            rms,
            blocks,
            stop,
            max_silent_blocks,
            worker: Some(worker),
        }
    }
}
impl Drop for Observer {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}
fn detect(
    observer: &Observer,
    predicate: impl Fn(f64) -> bool,
    start: Instant,
) -> Result<Duration> {
    let mut consecutive = 0;
    let mut last = observer.blocks.load(Ordering::Acquire);
    while start.elapsed() < Duration::from_secs(2) {
        let block = observer.blocks.load(Ordering::Acquire);
        if block != last {
            last = block;
            if predicate(f64::from_bits(observer.rms.load(Ordering::Acquire))) {
                consecutive += 1;
            } else {
                consecutive = 0;
            }
            if consecutive == 3 {
                return Ok(start.elapsed());
            }
        }
        thread::sleep(Duration::from_millis(1));
    }
    anyhow::bail!("Changed output signature not observed within two seconds")
}
