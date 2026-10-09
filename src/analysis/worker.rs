use super::models::{MODEL_ID, verify_models};
use crate::analysis_pcm::{MODEL_RATE, PREPROCESSING};
use anyhow::{Context, Result, ensure};
use beat_this::{BeatThis, RtenRuntime};
use serde::{Deserialize, Serialize};
use std::{
    fs::File,
    io::{BufWriter, Write},
    path::Path,
    time::Instant,
};
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Detection {
    pub frame: u64,
    pub score: f32,
    pub downbeat: bool,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Candidates {
    pub version: u32,
    pub song_key: String,
    pub map_revision: u64,
    pub rate: u32,
    pub frames: u64,
    pub model: String,
    pub preprocessing: String,
    pub elapsed_seconds: f64,
    pub beats: Vec<Detection>,
}
#[derive(Serialize, Deserialize)]
struct CachedAnalysis {
    pub candidates: Candidates,
    pub beat_logits: Vec<f32>,
    pub downbeat_logits: Vec<f32>,
}
pub fn read_result(path: &Path) -> Result<Candidates> {
    let file = File::open(path)?;
    ensure!(
        file.metadata()?.len() <= 32 * 1024 * 1024,
        "Analysis result exceeds cache limit"
    );
    let cache: CachedAnalysis = serde_json::from_reader(file)?;
    let result = cache.candidates;
    ensure!(
        result.version == 1 && result.model == MODEL_ID && result.preprocessing == PREPROCESSING,
        "Unsupported analysis cache"
    );
    ensure!(
        result.beats.len() <= 80_000
            && result.beats.windows(2).all(|w| w[0].frame < w[1].frame)
            && result
                .beats
                .iter()
                .all(|b| b.frame < result.frames && b.score.is_finite()),
        "Invalid analysis event positions"
    );
    Ok(result)
}
// Invoked in the same native executable before Tauri starts. Input is our own
// source-aligned mono WAV, never a second decode of the user's compressed file.
pub fn worker(args: &[String]) -> Result<()> {
    ensure!(args.len() == 7, "Invalid analysis worker arguments");
    let input = Path::new(&args[0]);
    let output = Path::new(&args[1]);
    let models = Path::new(&args[2]);
    let rate: u32 = args[4].parse()?;
    let frames: u64 = args[5].parse()?;
    let revision: u64 = args[6].parse()?;
    ensure!(rate > 0 && frames > 0, "Invalid analysis source");
    verify_models(models)?;
    let began = Instant::now();
    let mut tracker = BeatThis::new(
        &RtenRuntime,
        &models.join("mel_spectrogram.onnx"),
        &models.join("beat_this.onnx"),
    )?;
    let reader = hound::WavReader::open(input)?;
    ensure!(
        reader.spec().sample_rate == MODEL_RATE
            && reader.spec().channels == 1
            && reader.spec().sample_format == hound::SampleFormat::Float,
        "Invalid prepared analysis PCM"
    );
    let mut stream = tracker.stream(MODEL_RATE)?;
    let mut chunk = Vec::with_capacity(4096);
    for sample in reader.into_samples::<f32>() {
        let sample = sample?;
        ensure!(sample.is_finite(), "Non-finite analysis PCM");
        chunk.push(sample);
        if chunk.len() == 4096 {
            stream.push(&chunk)?;
            chunk.clear();
        }
    }
    if !chunk.is_empty() {
        stream.push(&chunk)?;
    }
    let analysis = stream.finish()?;
    let to_frame =
        |seconds: f32| ((f64::from(seconds) * f64::from(rate)).round() as u64).min(frames - 1);
    let downbeats: std::collections::HashSet<_> =
        analysis.downbeats.iter().map(|t| to_frame(*t)).collect();
    let mut beats = Vec::with_capacity(analysis.beats.len());
    for &seconds in &analysis.beats {
        ensure!(seconds.is_finite() && seconds >= 0.0, "Invalid model event");
        let frame = to_frame(seconds);
        let index = (seconds * 50.0).round() as usize;
        let logit = *analysis
            .beat_logits
            .get(index)
            .context("Model event has no activation")?;
        let score = 1.0 / (1.0 + (-logit).exp());
        beats.push(Detection {
            frame,
            score,
            downbeat: downbeats.contains(&frame),
        });
    }
    beats.dedup_by_key(|b| b.frame);
    let candidates = Candidates {
        version: 1,
        song_key: args[3].clone(),
        map_revision: revision,
        rate,
        frames,
        model: MODEL_ID.into(),
        preprocessing: PREPROCESSING.into(),
        elapsed_seconds: began.elapsed().as_secs_f64(),
        beats,
    };
    let cache = CachedAnalysis {
        candidates,
        beat_logits: analysis.beat_logits,
        downbeat_logits: analysis.downbeat_logits,
    };
    let mut writer = BufWriter::new(File::create(output)?);
    serde_json::to_writer(&mut writer, &cache)?;
    writer.flush()?;
    Ok(())
}
