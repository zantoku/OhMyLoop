//! Standalone stem-prototype PCM preparation, never part of playback.
use anyhow::{Context, Result, ensure};
use ohmyloop::audio::{Audio, write_wav};
use serde_json::json;
use sha2::{Digest, Sha256};
use std::{fs, io::Read, path::Path};

fn digest(path: &Path) -> Result<String> {
    let mut file = fs::File::open(path)?;
    let mut hash = Sha256::new();
    let mut buffer = [0u8; 65536];
    loop {
        let count = file.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        hash.update(&buffer[..count]);
    }
    Ok(format!("{:x}", hash.finalize()))
}

fn span(rate: u32, frames: usize, start: f64, duration: Option<f64>) -> Result<(usize, usize)> {
    ensure!(start.is_finite() && start >= 0.0, "Invalid start");
    let first = (start * rate as f64).floor() as usize;
    ensure!(first < frames, "Start outside recording");
    let end = if let Some(seconds) = duration {
        ensure!(seconds.is_finite() && seconds > 0.0, "Invalid duration");
        first
            .saturating_add((seconds * rate as f64).floor() as usize)
            .min(frames)
    } else {
        frames
    };
    ensure!(end > first, "Empty passage");
    Ok((first, end))
}

fn main() -> Result<()> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    ensure!(
        args.len() == 4,
        "Usage: stem_prepare INPUT NEW_OUTPUT_DIRECTORY START_SECONDS DURATION_SECONDS_OR_whole"
    );
    let source = Path::new(&args[0]);
    let output = Path::new(&args[1]);
    ensure!(!output.exists(), "Output directory already exists");
    let start = args[2].parse::<f64>().context("Invalid start seconds")?;
    let duration = if args[3] == "whole" {
        None
    } else {
        Some(args[3].parse::<f64>()?)
    };
    let hash = digest(source)?;
    let audio = Audio::load(source)?;
    ensure!(hash == digest(source)?, "Source changed during preparation");
    let (first, end) = span(audio.rate, audio.frames.len(), start, duration)?;
    let excerpt = Audio {
        rate: audio.rate,
        frames: audio.frames[first..end].to_vec(),
    }
    .resample(44100)?;
    fs::create_dir_all(output)?;
    let pcm = output.join("prepared.wav");
    write_wav(&pcm, excerpt.rate, &excerpt.frames)?;
    let metadata = json!({
        "version": 1, "preprocessing": "symphonia-stereo-rubato-delay-compensated-v1",
        "source_sha256": hash, "source_rate": audio.rate, "source_frames": audio.frames.len(),
        "source_span": { "start": first, "end_exclusive": end },
        "prepared_rate": excerpt.rate, "prepared_frames": excerpt.frames.len(),
        "prepared_sha256": digest(&pcm)?, "channels": 2,
        "geometry_policy": "floor(seconds * source_rate); floor(span_frames * 44100 / source_rate); no context"
    });
    fs::write(
        output.join("preparation.json"),
        serde_json::to_vec_pretty(&metadata)?,
    )?;
    println!("{}", output.display());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn source_span_rounding_clamping_and_invalid_controls() {
        assert_eq!(span(48000, 96000, 0.125, Some(0.5)).unwrap(), (6000, 30000));
        assert_eq!(span(48000, 96000, 1.5, Some(10.0)).unwrap(), (72000, 96000));
        assert!(span(48000, 96000, f64::NAN, None).is_err());
        assert!(span(48000, 96000, 2.0, None).is_err());
        assert!(span(48000, 96000, 0.0, Some(0.000001)).is_err());
    }
    #[test]
    fn resampled_excerpt_retains_expected_geometry() {
        let input = Audio::tone(48000, 0.25, 440.0);
        let output = input.resample(44100).unwrap();
        assert_eq!(output.frames.len(), 11025);
        assert!(output.frames.iter().flatten().all(|x| x.is_finite()));
    }
}
