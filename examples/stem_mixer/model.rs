// SPDX-License-Identifier: GPL-3.0-or-later
use anyhow::{Context, Result, ensure};
use ohmyloop::audio::Audio;
use serde_json::Value;
use std::{fs, path::Path};

pub const STEMS: [&str; 6] = ["bass", "drums", "guitar", "piano", "vocals", "other"];
const MEMORY_LIMIT: u64 = 2 * 1024 * 1024 * 1024;

pub struct Banks {
    pub audio: [Vec<Audio>; 2],
    pub labels: [String; 2],
    pub source_rate: u32,
    pub source_frames: usize,
    pub gain: f32,
    pub provenance_checked: bool,
}

impl Banks {
    pub fn load(dirs: [&Path; 2]) -> Result<Self> {
        let manifests = dirs
            .map(read_manifest)
            .into_iter()
            .collect::<Result<Vec<_>>>()?;
        if let [Some(a), Some(b)] = manifests.as_slice() {
            let a = &a["identity"]["preparation"];
            let b = &b["identity"]["preparation"];
            for key in [
                "source_sha256",
                "prepared_sha256",
                "source_rate",
                "source_span",
                "prepared_rate",
                "prepared_frames",
            ] {
                ensure!(
                    !a[key].is_null() && a[key] == b[key],
                    "Different or missing source preparation: {key}; use outputs from the same passage"
                );
            }
        }
        // Inspect every header before allocating any song-length PCM.
        let mut geometry = None;
        let mut bytes = 0_u64;
        for (bank, dir) in dirs.iter().enumerate() {
            for stem in STEMS {
                let path = dir.join(format!("{stem}.wav"));
                let reader = hound::WavReader::open(&path)
                    .with_context(|| format!("Opening {}", path.display()))?;
                let spec = reader.spec();
                let frames = reader.duration() as usize;
                ensure!(
                    spec.channels == 2 && spec.sample_rate > 0 && frames > 0,
                    "{} must be nonempty stereo WAV",
                    path.display()
                );
                let current = (spec.sample_rate, frames);
                ensure!(
                    geometry.is_none_or(|g| g == current),
                    "Rate/frame-count mismatch in {}",
                    path.display()
                );
                geometry = Some(current);
                bytes = bytes
                    .checked_add(frames as u64 * 8)
                    .context("PCM size overflow")?;
                if let Some(manifest) = &manifests[bank] {
                    let preparation = &manifest["identity"]["preparation"];
                    ensure!(
                        preparation["prepared_rate"].as_u64() == Some(spec.sample_rate as u64)
                            && preparation["prepared_frames"].as_u64() == Some(frames as u64),
                        "Manifest preparation geometry mismatch in {}",
                        dir.display()
                    );
                    let entry = &manifest["stems"][stem];
                    ensure!(
                        entry["file"].as_str() == Some(&format!("{stem}.wav"))
                            && entry["rate"].as_u64() == Some(spec.sample_rate as u64)
                            && entry["frames"].as_u64() == Some(frames as u64)
                            && entry["channels"].as_u64() == Some(2),
                        "Manifest geometry/name mismatch for {}",
                        path.display()
                    );
                }
            }
        }
        ensure!(
            bytes <= MEMORY_LIMIT,
            "PCM needs {:.2} GiB; this prototype limits both banks together to 2 GiB. Separate a shorter passage",
            bytes as f64 / 1024_f64.powi(3)
        );
        let (source_rate, source_frames) = geometry.context("No stems")?;
        let mut audio = [Vec::new(), Vec::new()];
        for (bank, dir) in dirs.iter().enumerate() {
            for stem in STEMS {
                eprintln!("Loading {} / {stem}", dir.display());
                audio[bank].push(read_wav(&dir.join(format!("{stem}.wav")))?);
            }
        }
        let mut result = Self {
            audio,
            labels: dirs.map(|dir| {
                dir.file_name()
                    .unwrap_or(dir.as_os_str())
                    .to_string_lossy()
                    .into_owned()
            }),
            source_rate,
            source_frames,
            gain: 1.0,
            provenance_checked: manifests.iter().all(Option::is_some),
        };
        result.measure_gain();
        Ok(result)
    }

    pub fn rate(&self) -> u32 {
        self.audio[0][0].rate
    }
    pub fn frames(&self) -> usize {
        self.audio[0][0].frames.len()
    }
    pub fn duration(&self) -> f64 {
        self.source_frames as f64 / self.source_rate as f64
    }

    pub fn resample(&mut self, rate: u32) -> Result<()> {
        if rate == self.rate() {
            return Ok(());
        }
        let frames = self.frames() as u64 * rate as u64 / self.rate() as u64;
        // Allow one old stem and resampler scratch in addition to final banks.
        ensure!(
            frames * 8 * 12 <= MEMORY_LIMIT,
            "Device-rate PCM exceeds 2 GiB; use a shorter passage"
        );
        for bank in &mut self.audio {
            for stem in bank {
                *stem = stem.resample(rate)?;
            }
        }
        // Resampling can overshoot; preserve the common original gain or reduce it.
        self.measure_gain();
        Ok(())
    }

    fn measure_gain(&mut self) {
        let mut bound = 0.0_f64;
        for bank in &self.audio {
            for frame in 0..bank[0].frames.len() {
                for channel in 0..2 {
                    let sum: f64 = bank
                        .iter()
                        .map(|s| f64::from(s.frames[frame][channel]).abs())
                        .sum();
                    bound = bound.max(sum);
                }
            }
        }
        self.gain = self.gain.min(if bound > 0.98 {
            (0.98 / bound) as f32
        } else {
            1.0
        });
    }

    pub fn mix(&self, bank: usize, muted: u8, frame: usize) -> [f32; 2] {
        let mut sum = [0.0_f64; 2];
        for (index, stem) in self.audio[bank].iter().enumerate() {
            if muted & (1 << index) == 0 {
                for (channel, value) in sum.iter_mut().enumerate() {
                    *value += f64::from(stem.frames[frame][channel]);
                }
            }
        }
        sum.map(|value| (value * f64::from(self.gain)) as f32)
    }
}

fn read_manifest(dir: &Path) -> Result<Option<Value>> {
    let path = dir.join("manifest.json");
    let bytes = match fs::read(&path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error).with_context(|| format!("Reading {}", path.display())),
    };
    let manifest: Value = serde_json::from_slice(&bytes).context("Invalid stem manifest JSON")?;
    ensure!(
        manifest["version"].as_u64() == Some(1) && manifest["status"].as_str() == Some("complete"),
        "Expected completed version-1 manifest in {}",
        dir.display()
    );
    Ok(Some(manifest))
}

fn read_wav(path: &Path) -> Result<Audio> {
    let mut reader = hound::WavReader::open(path)?;
    let spec = reader.spec();
    let count = reader.duration() as usize;
    let mut frames = Vec::with_capacity(count);
    match spec.sample_format {
        hound::SampleFormat::Float => {
            let mut samples = reader.samples::<f32>();
            while let Some(left) = samples.next() {
                let pair = [left?, samples.next().context("Odd stereo sample count")??];
                ensure!(
                    pair.iter().all(|sample| sample.is_finite()),
                    "Nonfinite PCM in {}",
                    path.display()
                );
                frames.push(pair);
            }
        }
        hound::SampleFormat::Int => {
            ensure!(
                (1..=32).contains(&spec.bits_per_sample),
                "Unsupported PCM bit depth"
            );
            let scale = 2_f64.powi(i32::from(spec.bits_per_sample) - 1);
            let mut samples = reader.samples::<i32>();
            while let Some(left) = samples.next() {
                frames.push([
                    (f64::from(left?) / scale) as f32,
                    (f64::from(samples.next().context("Odd stereo sample count")??) / scale) as f32,
                ]);
            }
        }
    }
    ensure!(frames.len() == count, "Truncated WAV in {}", path.display());
    Ok(Audio {
        rate: spec.sample_rate,
        frames,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generated_wav_banks_reject_geometry_nonfinite_and_different_passages() {
        let root = std::env::temp_dir().join(format!("ohmyloop-stem-mixer-{}", std::process::id()));
        let dirs = [root.join("a"), root.join("b")];
        for dir in &dirs {
            fs::create_dir_all(dir).unwrap();
            for name in STEMS {
                write_test_wav(&dir.join(format!("{name}.wav")), 1000, 0.1);
            }
        }
        let load = || Banks::load([&dirs[0], &dirs[1]]);
        assert!(!load().unwrap().provenance_checked);
        let guitar = dirs[1].join("guitar.wav");
        write_test_wav(&guitar, 2000, 0.1);
        assert!(
            load()
                .err()
                .unwrap()
                .to_string()
                .contains("Rate/frame-count mismatch")
        );
        write_test_wav(&guitar, 1000, f32::NAN);
        assert!(load().err().unwrap().to_string().contains("Nonfinite"));
        write_test_wav(&guitar, 1000, 0.1);
        for (index, dir) in dirs.iter().enumerate() {
            let manifest = serde_json::json!({ "version": 1, "status": "complete", "identity": { "preparation": { "source_sha256": format!("source-{index}") } } });
            fs::write(
                dir.join("manifest.json"),
                serde_json::to_vec(&manifest).unwrap(),
            )
            .unwrap();
        }
        assert!(load().err().unwrap().to_string().contains("source_sha256"));
        fs::remove_dir_all(root).unwrap();
    }

    fn write_test_wav(path: &Path, rate: u32, value: f32) {
        let spec = hound::WavSpec {
            channels: 2,
            sample_rate: rate,
            bits_per_sample: 32,
            sample_format: hound::SampleFormat::Float,
        };
        let mut writer = hound::WavWriter::create(path, spec).unwrap();
        for _ in 0..20 {
            writer.write_sample(value).unwrap();
        }
        writer.finalize().unwrap();
    }

    #[test]
    fn every_mute_mask_has_joint_headroom_without_model_normalization() {
        let mut banks = Banks {
            audio: [
                vec![
                    Audio {
                        rate: 1000,
                        frames: vec![[0.8, -0.7]; 4]
                    };
                    6
                ],
                vec![
                    Audio {
                        rate: 1000,
                        frames: vec![[0.4, -0.35]; 4]
                    };
                    6
                ],
            ],
            labels: ["A".into(), "B".into()],
            source_rate: 1000,
            source_frames: 4,
            gain: 1.0,
            provenance_checked: false,
        };
        banks.measure_gain();
        for mask in 0..64 {
            for bank in 0..2 {
                assert!(banks.mix(bank, mask, 0).iter().all(|s| s.abs() <= 0.980001));
            }
        }
        assert_eq!(banks.mix(0, 63, 0), [0.0; 2]);
        assert_eq!(banks.mix(0, 0, 0)[0], banks.mix(1, 0, 0)[0] * 2.0);
    }
}
