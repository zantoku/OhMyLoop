//! Full-reference preparation shared by the desktop worker and CLI wrapper.
use super::WorkControl as Control;
use anyhow::{Result, ensure};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::io::Read;
use std::{fs, path::Path};
#[derive(Clone, Debug, PartialEq)]
pub struct Freshness {
    pub bytes: u64,
    pub modified: std::time::SystemTime,
}
pub fn freshness(path: &Path) -> Result<Freshness> {
    let m = fs::metadata(path)?;
    ensure!(m.is_file(), "Reference is not a file");
    Ok(Freshness {
        bytes: m.len(),
        modified: m.modified()?,
    })
}
pub fn prepare_reference(
    reference: &Path,
    owned: &Path,
    start: f64,
    duration: Option<f64>,
    expected: Option<(u32, u64)>,
    control: &Control,
    mut report: impl FnMut(&str, u64, Option<u64>),
) -> Result<(Value, Freshness)> {
    let initial = freshness(reference)?;
    report("source.hash", 0, None);
    let hash = digest(reference, control, |n, t| report("source.hash", n, Some(t)))?;
    report("source.decode", 0, expected.map(|(_, frames)| frames));
    let audio = crate::audio::Audio::load_checked(reference, |n| {
        control.check()?;
        report("source.decode", n, expected.map(|(_, frames)| frames));
        Ok(())
    })?;
    let rate = audio.rate;
    let frames = audio.frames.len() as u64;
    ensure!(
        expected.is_none_or(|geometry| geometry == (rate, frames)),
        "Reference geometry changed"
    );
    let (first, end) = span(rate, audio.frames.len(), start, duration)?;
    ensure!(
        freshness(reference)? == initial,
        "Reference changed during preparation"
    );
    report("source.check", 0, None);
    let endhash = digest(reference, control, |n, t| {
        report("source.check", n, Some(t))
    })?;
    ensure!(hash == endhash, "Reference changed during preparation");
    let count = crate::stems::coordinate_frames((end - first) as u64, 44100, rate)?;
    let audio = if first == 0 && end == audio.frames.len() {
        audio
    } else {
        crate::audio::Audio {
            rate,
            frames: audio.frames[first..end].to_vec(),
        }
    };
    report("source.resample", 0, Some(count));
    let audio = if audio.rate == 44100 {
        control.check()?;
        audio
    } else {
        audio.resample_checked(44100, |n| {
            control.check()?;
            report("source.resample", n, Some(count));
            Ok(())
        })?
    };
    report("source.write", 0, Some(count));
    fs::create_dir_all(owned)?;
    let target = owned.join("prepared.wav");
    let mut writer = hound::WavWriter::create(
        &target,
        hound::WavSpec {
            channels: 2,
            sample_rate: 44100,
            bits_per_sample: 32,
            sample_format: hound::SampleFormat::Float,
        },
    )?;
    for (index, block) in audio.frames.chunks(8192).enumerate() {
        control.check()?;
        for sample in block.iter().flatten() {
            writer.write_sample(*sample)?;
        }
        report(
            "source.write",
            (index * 8192 + block.len()) as u64,
            Some(count),
        );
    }
    writer.finalize()?;
    report("source.pcm", 0, None);
    let prepared_hash = digest(&target, control, |n, t| report("source.pcm", n, Some(t)))?;
    ensure!(
        freshness(reference)? == initial,
        "Reference changed during preparation"
    );
    Ok((
        json!({"version":1,"preprocessing":"symphonia-stereo-rubato-delay-compensated-v1","geometry_policy":"floor(seconds * source_rate); floor(span_frames * 44100 / source_rate); no context","source_sha256":hash,"source_rate":rate,"source_frames":frames,"source_span":{"start":first,"end_exclusive":end},"prepared_rate":44100,"prepared_frames":count,"prepared_sha256":prepared_hash,"channels":2}),
        initial,
    ))
}

pub fn span(rate: u32, frames: usize, start: f64, duration: Option<f64>) -> Result<(usize, usize)> {
    ensure!(start.is_finite() && start >= 0., "Invalid start");
    let first = (start * rate as f64).floor() as usize;
    ensure!(first < frames, "Start outside recording");
    let end = if let Some(seconds) = duration {
        ensure!(seconds.is_finite() && seconds > 0., "Invalid duration");
        first
            .saturating_add((seconds * rate as f64).floor() as usize)
            .min(frames)
    } else {
        frames
    };
    ensure!(end > first, "Empty passage");
    Ok((first, end))
}
fn digest(path: &Path, control: &Control, mut report: impl FnMut(u64, u64)) -> Result<String> {
    let mut file = fs::File::open(path)?;
    let total = file.metadata()?.len();
    let mut hash = Sha256::new();
    let mut b = vec![0; 256 * 1024];
    let mut n = 0;
    loop {
        control.check()?;
        let count = file.read(&mut b)?;
        if count == 0 {
            break;
        }
        hash.update(&b[..count]);
        n += count as u64;
        report(n, total);
    }
    Ok(format!("{:x}", hash.finalize()))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn full_and_excerpt_share_policy_with_bounded_cooperative_cancel() {
        let root = std::env::temp_dir().join(uuid::Uuid::new_v4().to_string());
        fs::create_dir(&root).unwrap();
        let source = root.join("source.wav");
        crate::audio::write_wav(
            &source,
            48000,
            &crate::audio::Audio::tone(48000, 1., 440.).frames,
        )
        .unwrap();
        let (prep, _) = prepare_reference(
            &source,
            &root.join("full"),
            0.,
            None,
            Some((48000, 48000)),
            &Control::default(),
            |_, _, _| {},
        )
        .unwrap();
        assert_eq!(prep["prepared_frames"], 44100);
        let (prep, _) = prepare_reference(
            &source,
            &root.join("excerpt"),
            0.125,
            Some(0.5),
            None,
            &Control::default(),
            |_, _, _| {},
        )
        .unwrap();
        assert_eq!(prep["source_span"]["start"], 6000);
        assert_eq!(prep["prepared_frames"], 22050);
        for stage in [
            "source.hash",
            "source.decode",
            "source.resample",
            "source.write",
            "source.pcm",
        ] {
            let control = Control::default();
            let result = prepare_reference(
                &source,
                &root.join(uuid::Uuid::new_v4().to_string()),
                0.,
                None,
                None,
                &control,
                |s, _, _| {
                    if s == stage {
                        control.cancel();
                    }
                },
            );
            assert!(result.is_err(), "{stage}");
        }
        fs::remove_dir_all(root).unwrap();
    }
}
