// SPDX-License-Identifier: GPL-3.0-or-later
use anyhow::Result;
use serde_json::json;
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::Read,
    path::{Path, PathBuf},
};
pub struct Fixture {
    pub root: PathBuf,
    pub reference: PathBuf,
    pub sets: Vec<PathBuf>,
}
impl Fixture {
    pub fn new(seconds: u32) -> Result<Self> {
        let root = std::env::temp_dir().join(format!(
            "ohmyloop-stem-benchmark-{}-{}",
            std::process::id(),
            uuid::Uuid::new_v4()
        ));
        fs::create_dir(&root)?;
        let guard = Self {
            reference: root.join("reference.wav"),
            sets: vec![root.join("A"), root.join("B")],
            root,
        };
        let reference_rate = 44_100;
        let stem_rate = 48_000;
        write_tone(&guard.reference, reference_rate, seconds, 0.18)?;
        let digest = hash(&guard.reference)?;
        for (set_index, directory) in guard.sets.iter().enumerate() {
            fs::create_dir(directory)?;
            let mut stems = serde_json::Map::new();
            let prepared_path = directory.join("prepared.wav");
            write_tone(&prepared_path, stem_rate, seconds, 0.18)?;
            for role in ohmyloop::stems::Role::ALL {
                let name = format!("{}.wav", role.name());
                let path = directory.join(&name);
                let amplitude = if role.index() == 0 {
                    if set_index == 0 { 0.12 } else { 0.03 }
                } else {
                    0.0
                };
                write_tone(&path, stem_rate, seconds, amplitude)?;
                stems.insert(role.name().into(), json!({"file":name,"rate":stem_rate,"frames":u64::from(seconds)*u64::from(stem_rate),"channels":2,"sha256":hash(&path)?}));
            }
            let reference_frames = u64::from(seconds) * u64::from(reference_rate);
            let mut manifest = json!({"version":1,"status":"complete","identity":{"preparation":{"version":1,"source_sha256":digest,"source_rate":reference_rate,"source_frames":reference_frames,"source_span":{"start":0,"end_exclusive":reference_frames},"prepared_sha256":hash(&prepared_path)?,"prepared_rate":stem_rate,"prepared_frames":u64::from(seconds)*u64::from(stem_rate),"channels":2,"geometry_policy":"floor(seconds * source_rate); floor(span_frames * 44100 / source_rate); no context"}},"stems":stems});
            manifest["job_key"] = json!(format!(
                "{:x}",
                Sha256::digest(serde_json::to_vec(&manifest["identity"])?)
            ));
            fs::write(
                directory.join("manifest.json"),
                serde_json::to_vec_pretty(&manifest)?,
            )?;
        }
        Ok(guard)
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}
fn write_tone(path: &Path, rate: u32, seconds: u32, amplitude: f32) -> Result<()> {
    let spec = hound::WavSpec {
        channels: 2,
        sample_rate: rate,
        bits_per_sample: 32,
        sample_format: hound::SampleFormat::Float,
    };
    let mut writer = hound::WavWriter::create(path, spec)?;
    for index in 0..u64::from(seconds) * u64::from(rate) {
        let sample = amplitude
            * (index as f64 * 330.0 * std::f64::consts::TAU / f64::from(rate)).sin() as f32;
        writer.write_sample(sample)?;
        writer.write_sample(sample)?;
    }
    writer.finalize()?;
    Ok(())
}
fn hash(path: &Path) -> Result<String> {
    let mut file = fs::File::open(path)?;
    let mut buffer = [0; 65536];
    let mut digest = Sha256::new();
    loop {
        let count = file.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        digest.update(&buffer[..count]);
    }
    Ok(format!("{:x}", digest.finalize()))
}
