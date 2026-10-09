//! Source-aligned analysis PCM preparation, independent of inference and playback.
use crate::audio::Audio;
use anyhow::{Result, ensure};
use rubato::{
    Resampler, SincFixedIn, SincInterpolationParameters, SincInterpolationType, WindowFunction,
};
use std::{
    path::Path,
    sync::atomic::{AtomicBool, Ordering},
};
pub const MODEL_RATE: u32 = 22_050;
pub const PREPROCESSING: &str = "mono-average-rubato016-phase-align-v1";

pub fn write_model_pcm(audio: &Audio, path: &Path, cancelled: &AtomicBool) -> Result<()> {
    ensure!(
        audio.rate > 0 && !audio.frames.is_empty(),
        "No source audio to analyze"
    );
    let target = ((audio.frames.len() as u64 * u64::from(MODEL_RATE) + u64::from(audio.rate) / 2)
        / u64::from(audio.rate)) as usize;
    ensure!(target > 0, "Audio is too short to analyze");
    let mut writer = hound::WavWriter::create(
        path,
        hound::WavSpec {
            channels: 1,
            sample_rate: MODEL_RATE,
            bits_per_sample: 32,
            sample_format: hound::SampleFormat::Float,
        },
    )?;
    let mono = |frame: &[f32; 2]| (frame[0] + frame[1]) * 0.5;
    if audio.rate == MODEL_RATE {
        for chunk in audio.frames.chunks(4096) {
            ensure!(!cancelled.load(Ordering::Acquire), "Analysis cancelled");
            for frame in chunk {
                let sample = mono(frame);
                ensure!(sample.is_finite(), "Invalid source audio");
                writer.write_sample(sample)?;
            }
        }
    } else {
        let mut converter = SincFixedIn::<f32>::new(
            MODEL_RATE as f64 / audio.rate as f64,
            1.0,
            SincInterpolationParameters {
                sinc_len: 256,
                f_cutoff: 0.95,
                interpolation: SincInterpolationType::Cubic,
                oversampling_factor: 256,
                window: WindowFunction::BlackmanHarris2,
            },
            4096,
            1,
        )?;
        let mut input = vec![vec![0.0; 4096]];
        let mut cursor = 0;
        let mut written = 0;
        // SincFixedIn 0.16 starts at -L/2 and advances one output step
        // before evaluation. The polyphase kernel reverses the subphase, so
        // its actual origin differs from the advertised pipeline latency.
        // Align that origin to source frame zero; impulse tests guard this
        // dependency-specific convention to within one model-rate sample.
        let ratio = f64::from(MODEL_RATE) / f64::from(audio.rate);
        let origin = ((1.0 - 1.0 / 256.0) * ratio - 1.0).round() as i64;
        let mut discard = origin.max(0) as usize;
        for _ in 0..(-origin).max(0) as usize {
            writer.write_sample(0.0_f32)?;
            written += 1;
        }
        let max_runs = audio.frames.len().div_ceil(4096) + 8;
        for _ in 0..max_runs {
            ensure!(!cancelled.load(Ordering::Acquire), "Analysis cancelled");
            for (n, sample) in input[0].iter_mut().enumerate() {
                *sample = audio.frames.get(cursor + n).map_or(0.0, mono);
                ensure!(sample.is_finite(), "Invalid source audio");
            }
            cursor += 4096;
            let output = converter.process(&input, None)?;
            for &sample in &output[0] {
                if discard > 0 {
                    discard -= 1;
                    continue;
                }
                if written == target {
                    break;
                }
                writer.write_sample(sample)?;
                written += 1;
            }
            if written == target {
                break;
            }
        }
        ensure!(
            written == target,
            "Analysis resampler did not flush the source tail"
        );
    }
    writer.finalize()?;
    Ok(())
}
