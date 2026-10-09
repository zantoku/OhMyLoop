//! Reproducible generated-audio checks for the optional real inference worker.
use anyhow::{Context, Result, ensure};
use ohmyloop::{
    analysis::AnalysisManager,
    audio::{Audio, write_wav},
    playback::Player,
};
use serde_json::json;
use std::{
    fs,
    path::PathBuf,
    sync::{Arc, atomic::Ordering},
    thread,
    time::{Duration, Instant},
};

fn fixture(variable: bool) -> (Audio, Vec<f64>, Vec<f64>) {
    let rate = 48_000;
    let mut times = vec![];
    let mut downbeats = vec![];
    let mut at = 1.0;
    for beat in 0..64 {
        times.push(at);
        if (!variable && beat % 4 == 0)
            || (variable
                && (beat < 32 && beat % 4 == 0
                    || (32..50).contains(&beat) && (beat - 32) % 3 == 0
                    || beat >= 50 && (beat - 50) % 4 == 0))
        {
            downbeats.push(at);
        }
        at += if !variable {
            0.5
        } else if beat < 24 {
            0.6
        } else if beat < 40 {
            0.4
        } else {
            0.5 + 0.06 * (f64::from(beat) * 0.35).sin()
        };
    }
    let frames = (at * f64::from(rate)).ceil() as usize;
    let mut audio = Audio {
        rate,
        frames: vec![[0.0; 2]; frames],
    };
    for (index, &start) in times.iter().enumerate() {
        let accent = downbeats.contains(&start);
        let base = (start * f64::from(rate)).round() as usize;
        for n in 0..(rate / 3) as usize {
            if base + n >= frames {
                break;
            }
            let t = n as f64 / f64::from(rate);
            let kick = (std::f64::consts::TAU
                * (70.0 * t + 25.0 * (1.0 - (-t * 30.0).exp()) / 30.0))
                .sin()
                * (-t * 25.0).exp();
            let root = [110.0, 146.832, 130.813, 98.0][index / 8 % 4];
            let chord = (std::f64::consts::TAU * root * t).sin()
                + 0.6 * (std::f64::consts::TAU * root * 1.5 * t).sin();
            let noise = ((n as u32).wrapping_mul(1664525).wrapping_add(1013904223) >> 8) as f64
                / 16777216.0
                * 2.0
                - 1.0;
            let sample = (0.28 * kick
                + 0.09 * chord * (-t * 10.0).exp()
                + 0.13 * noise * (-t * 100.0).exp())
                * if accent { 1.0 } else { 0.65 };
            audio.frames[base + n] = [sample as f32, sample as f32];
        }
    }
    (audio, times, downbeats)
}
fn score(predicted: &[f64], truth: &[f64], tolerance: f64) -> serde_json::Value {
    let mut used = vec![false; truth.len()];
    let mut errors = vec![];
    for &p in predicted {
        if let Some((index, &t)) = truth
            .iter()
            .enumerate()
            .filter(|(i, t)| !used[*i] && (**t - p).abs() <= tolerance + 1e-9)
            .min_by(|(_, a), (_, b)| (**a - p).abs().total_cmp(&(**b - p).abs()))
        {
            used[index] = true;
            errors.push((t - p).abs());
        }
    }
    errors.sort_by(f64::total_cmp);
    let matched = errors.len();
    json!({"truth":truth.len(),"predicted":predicted.len(),"matched":matched,"precision":matched as f64/predicted.len().max(1) as f64,"recall":matched as f64/truth.len().max(1) as f64,"f1":2.0*matched as f64/(predicted.len()+truth.len()).max(1) as f64,"matched_median_ms":errors.get(errors.len()/2).map(|e|e*1000.0),"matched_max_ms":errors.last().map(|e|e*1000.0)})
}
fn wait(manager: &AnalysisManager) -> Result<ohmyloop::analysis::JobStatus> {
    let begin = Instant::now();
    let mut previous = String::new();
    loop {
        let status = manager.status();
        let progress =
            serde_json::to_string(&json!({"phase":status.phase,"progress":status.progress}))?;
        if status.running && progress != previous {
            println!("{progress}");
            previous = progress;
        }
        if !status.running {
            return Ok(status);
        }
        ensure!(
            begin.elapsed() < Duration::from_secs(120),
            "Analysis probe timed out"
        );
        thread::sleep(Duration::from_millis(25));
    }
}
fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let executable = PathBuf::from(args.first().context(
        "Usage: analysis_probe worker-executable model-directory output-directory [--device] [--install]",
    )?)
    .canonicalize()?;
    let models = PathBuf::from(args.get(1).context("Missing models directory")?);
    fs::create_dir_all(&models)?;
    let models = models.canonicalize()?;
    let directory = PathBuf::from(args.get(2).context("Missing output directory")?);
    fs::create_dir_all(&directory)?;
    let manager = AnalysisManager::new(models, directory.join("cache"))?;
    let installation = if args.iter().any(|a| a == "--install") {
        let began = Instant::now();
        manager.install()?;
        let status = wait(&manager)?;
        ensure!(
            status.error.is_none(),
            "{}",
            status.error.unwrap_or_default()
        );
        ensure!(status.model_ready, "Installed model is not ready");
        Some(json!({"seconds": began.elapsed().as_secs_f64(), "verified": true}))
    } else {
        None
    };
    let mut reports = vec![];
    for (name, variable) in [("steady-4-4", false), ("tempo-meter-rubato", true)] {
        let (audio, truth, downbeat_truth) = fixture(variable);
        write_wav(
            &directory.join(format!("{name}.wav")),
            audio.rate,
            &audio.frames,
        )?;
        let audio = Arc::new(audio);
        let player = if args.iter().any(|a| a == "--device") {
            let p = Player::open_shared(audio.clone(), true)?;
            p.play()?;
            Some(p)
        } else {
            None
        };
        let started = Instant::now();
        manager.analyze(executable.clone(), audio.clone(), name.into(), 0)?;
        let status = wait(&manager)?;
        ensure!(
            status.error.is_none(),
            "{}",
            status.error.unwrap_or_default()
        );
        let result = status.result.context("Missing model output")?;
        let beats: Vec<_> = result
            .beats
            .iter()
            .map(|b| b.frame as f64 / f64::from(result.rate))
            .collect();
        let downbeats: Vec<_> = result
            .beats
            .iter()
            .filter(|b| b.downbeat)
            .map(|b| b.frame as f64 / f64::from(result.rate))
            .collect();
        let report = json!({"case":name,"wall_seconds":started.elapsed().as_secs_f64(),"model_seconds":result.elapsed_seconds,"beat_70ms":score(&beats,&truth,0.07),"beat_20ms":score(&beats,&truth,0.02),"downbeat_70ms":score(&downbeats,&downbeat_truth,0.07),"source_frames":audio.frames.len(),"device_errors":player.as_ref().map(|p|p.shared.device_errors.load(Ordering::Relaxed)),"underrun_events":player.as_ref().map(|p|p.shared.underflow_frames.load(Ordering::Relaxed)),"callbacks":player.as_ref().map(|p|p.shared.callbacks.load(Ordering::Relaxed)),"max_render_ms":player.as_ref().map(|p|p.shared.max_render_us.load(Ordering::Relaxed) as f64 / 1000.0)});
        if let Some(p) = player {
            p.pause();
        }
        println!("{report}");
        reports.push(report);
    }
    let audio = Arc::new(fixture(false).0);
    let start = manager.analyze(executable, audio, "cancel-case".into(), 0)?;
    let begin = Instant::now();
    while manager.worker_pid().is_none() {
        ensure!(
            begin.elapsed() < Duration::from_secs(10),
            "Worker failed to start"
        );
        thread::sleep(Duration::from_millis(10));
    }
    thread::sleep(Duration::from_millis(100));
    let cancelled = Instant::now();
    manager.cancel(start.id);
    let stopped = wait(&manager)?;
    ensure!(
        stopped.phase == "Cancelled" && stopped.result.is_none(),
        "Cancelled analysis produced a result"
    );
    let cancellation = json!({"cancel_seconds":cancelled.elapsed().as_secs_f64(),"result_discarded":stopped.result.is_none()});
    println!("{cancellation}");
    fs::write(
        directory.join("report.json"),
        serde_json::to_vec_pretty(
            &json!({"cases":reports,"installation":installation,"cancellation":cancellation,"limits":"Generated recordings only; this is not Python parity or general music accuracy."}),
        )?,
    )?;
    manager.shutdown();
    Ok(())
}
