//! Standalone stem-prototype PCM preparation, never part of playback.
use anyhow::{Context, Result, ensure};
#[cfg(test)]
use ohmyloop::audio::Audio;
use std::{fs, path::Path};

#[cfg(test)]
use ohmyloop::stems::source::span;

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
    let (metadata, _) = ohmyloop::stems::source::prepare_reference(
        source,
        output,
        start,
        duration,
        None,
        &ohmyloop::stems::WorkControl::default(),
        |_, _, _| {},
    )?;
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
