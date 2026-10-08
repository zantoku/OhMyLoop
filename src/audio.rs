use anyhow::{Context, Result, bail, ensure};
use std::{fs::File, io::ErrorKind, path::Path};
use symphonia::core::{
    audio::SampleBuffer, codecs::DecoderOptions, errors::Error, formats::FormatOptions,
    io::MediaSourceStream, meta::MetadataOptions, probe::Hint,
};

/// The prototype decodes one song into stereo PCM. Coordinates use this rate.
#[derive(Clone)]
pub struct Audio {
    pub rate: u32,
    pub frames: Vec<[f32; 2]>,
}

impl Audio {
    pub fn duration(&self) -> f64 {
        self.frames.len() as f64 / self.rate as f64
    }

    pub fn load(path: &Path) -> Result<Self> {
        let stream = MediaSourceStream::new(Box::new(File::open(path)?), Default::default());
        let mut hint = Hint::new();
        if let Some(extension) = path.extension().and_then(|x| x.to_str()) {
            hint.with_extension(extension);
        }
        let mut format = symphonia::default::get_probe()
            .format(
                &hint,
                stream,
                &FormatOptions::default(),
                &MetadataOptions::default(),
            )?
            .format;
        let track = format.default_track().context("No default audio track")?;
        let id = track.id;
        let mut decoder = symphonia::default::get_codecs()
            .make(&track.codec_params, &DecoderOptions::default())?;
        let mut frames = Vec::new();
        let mut rate = None;
        loop {
            let packet = match format.next_packet() {
                Ok(packet) => packet,
                Err(Error::IoError(error)) if error.kind() == ErrorKind::UnexpectedEof => break,
                Err(error) => return Err(error.into()),
            };
            if packet.track_id() != id {
                continue;
            }
            // Reject corrupt packets instead of silently hiding decode gaps.
            let decoded = decoder.decode(&packet)?;
            let spec = *decoded.spec();
            ensure!(
                rate.is_none_or(|r| r == spec.rate),
                "Sample rate changed mid-file"
            );
            rate = Some(spec.rate);
            let channels = spec.channels.count();
            ensure!(
                channels == 1 || channels == 2,
                "Prototype supports mono/stereo files"
            );
            let mut buffer = SampleBuffer::<f32>::new(decoded.capacity() as u64, spec);
            buffer.copy_interleaved_ref(decoded);
            for sample in buffer.samples().chunks_exact(channels) {
                ensure!(
                    sample.iter().all(|x| x.is_finite()),
                    "Non-finite audio sample"
                );
                frames.push([sample[0], sample[channels - 1]]);
            }
            // ~512 MiB PCM cap; avoids exhausting memory on long recordings.
            ensure!(
                frames.len() <= 64_000_000,
                "Song exceeds prototype's 512 MiB PCM limit"
            );
        }
        ensure!(!frames.is_empty(), "Audio file is empty");
        let rate = rate.context("No decoded sample rate")?;
        ensure!(rate > 0, "Invalid sample rate");
        Ok(Self { rate, frames })
    }

    /// Simple offline rate conversion, only used when the device rejects the
    /// source rate. This is explicitly not production-quality resampling.
    pub fn resample_linear(&self, rate: u32) -> Result<Self> {
        ensure!(rate > 0, "Invalid output sample rate");
        if rate == self.rate {
            return Ok(self.clone());
        }
        let count = (self.frames.len() as u64 * rate as u64 / self.rate as u64) as usize;
        ensure!(
            count > 0 && count <= 64_000_000,
            "Resampled audio exceeds memory limit"
        );
        let frames = (0..count)
            .map(|n| {
                let p = n as f64 * self.rate as f64 / rate as f64;
                let a = p.floor() as usize;
                let b = (a + 1).min(self.frames.len() - 1);
                let fraction = (p - a as f64) as f32;
                std::array::from_fn(|c| {
                    self.frames[a][c] * (1.0 - fraction) + self.frames[b][c] * fraction
                })
            })
            .collect();
        Ok(Self { rate, frames })
    }

    /// Original synthetic practice material: changing chords, bass, and clicks.
    pub fn demo() -> Self {
        let rate = 48_000;
        let frames = (0..rate as usize * 12)
            .map(|n| {
                let t = n as f64 / rate as f64;
                let root = [220.0, 174.614, 195.998, 164.814][(t / 3.0) as usize % 4];
                let beat = t % 0.5;
                let envelope = (-5.0 * (t % 0.5)).exp();
                let sine = |f: f64| (std::f64::consts::TAU * f * t).sin();
                let chord = 0.08 * envelope * (sine(root) + sine(root * 1.25) + sine(root * 1.5));
                let bass = 0.09 * sine(root / 2.0);
                let click = if beat < 0.015 {
                    0.12 * (-beat * 350.0).exp() * sine(1800.0)
                } else {
                    0.0
                };
                [
                    (chord + bass + click) as f32,
                    (chord * 0.9 + bass + click) as f32,
                ]
            })
            .collect();
        Self { rate, frames }
    }

    pub fn tone(rate: u32, seconds: f64, hz: f64) -> Self {
        Self {
            rate,
            frames: (0..(seconds * rate as f64) as usize)
                .map(|n| {
                    let value =
                        (0.2 * (std::f64::consts::TAU * hz * n as f64 / rate as f64).sin()) as f32;
                    [value, value]
                })
                .collect(),
        }
    }
}

pub fn write_wav(path: &Path, rate: u32, frames: &[[f32; 2]]) -> Result<()> {
    if frames.iter().flatten().any(|x| !x.is_finite()) {
        bail!("Cannot write non-finite audio");
    }
    let mut writer = hound::WavWriter::create(
        path,
        hound::WavSpec {
            channels: 2,
            sample_rate: rate,
            bits_per_sample: 32,
            sample_format: hound::SampleFormat::Float,
        },
    )?;
    for frame in frames {
        for &sample in frame {
            writer.write_sample(sample)?;
        }
    }
    writer.finalize()?;
    Ok(())
}
