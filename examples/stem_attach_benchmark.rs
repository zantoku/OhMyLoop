// SPDX-License-Identifier: GPL-3.0-or-later
//! Generated-audio attachment timing and processing-pass diagnostics. No user audio.
use anyhow::{Result, ensure};
use ohmyloop::{
    project::asset_digest,
    stems::{PreparationProgress, Role, library},
};
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    time::Instant,
};
struct Fixture(PathBuf);
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
fn wav(path: &Path, seconds: u32, frequency: f64) -> Result<()> {
    let spec = hound::WavSpec {
        channels: 2,
        sample_rate: 44100,
        bits_per_sample: 32,
        sample_format: hound::SampleFormat::Float,
    };
    let mut writer = hound::WavWriter::create(path, spec)?;
    for i in 0..u64::from(seconds) * 44100 {
        let value = (0.08 * (std::f64::consts::TAU * frequency * i as f64 / 44100.).sin()) as f32;
        writer.write_sample(value)?;
        writer.write_sample(-value)?;
    }
    writer.finalize()?;
    Ok(())
}
fn fixture(seconds: u32) -> Result<Fixture> {
    let root = std::env::temp_dir().join(format!(
        "ohmyloop-attach-benchmark-{}",
        uuid::Uuid::new_v4()
    ));
    fs::create_dir(&root)?;
    let root = root.canonicalize()?;
    let prototype = root.join("prototype");
    fs::create_dir(&prototype)?;
    let reference = root.join("reference.wav");
    wav(&reference, seconds, 440.)?;
    fs::copy(&reference, prototype.join("prepared.wav"))?;
    let frames = u64::from(seconds) * 44100;
    let hash = asset_digest(&reference)?;
    let identity = serde_json::json!({"preparation":{"version":1,"source_sha256":hash,"source_rate":44100,"source_frames":frames,"source_span":{"start":0,"end_exclusive":frames},"prepared_rate":44100,"prepared_frames":frames,"channels":2,"prepared_sha256":hash,"preprocessing":"symphonia-stereo-rubato-delay-compensated-v1","geometry_policy":"floor(seconds * source_rate); floor(span_frames * 44100 / source_rate); no context"},"settings":{"normalization":"none per stem"},"model_lock":{"version":1,"model":"demucs-6s","files":{"5c90dfd2-34c22ccb.th":{"bytes":1,"sha256":format!("34c22ccb{}","0".repeat(56)),"url":"https://example.test/generated-fixture-weights"},"htdemucs_6s.yaml":{"bytes":1,"sha256":"0".repeat(64),"url":"https://example.test/generated-fixture-config"}}}});
    let mut stems = serde_json::Map::new();
    for role in Role::ALL {
        let name = format!("{}.wav", role.name());
        let path = prototype.join(&name);
        wav(&path, seconds, 220. + role.index() as f64 * 110.)?;
        stems.insert(role.name().into(),serde_json::json!({"file":name,"sha256":asset_digest(&path)?,"rate":44100,"frames":frames,"channels":2}));
    }
    use sha2::{Digest, Sha256};
    let manifest = serde_json::json!({"version":1,"status":"complete","identity":identity,"job_key":format!("{:x}",Sha256::digest(serde_json::to_vec(&identity)?)),"corrections":[],"stems":stems});
    fs::write(
        prototype.join("manifest.json"),
        serde_json::to_vec(&manifest)?,
    )?;
    Ok(Fixture(root))
}
#[derive(Default)]
struct Stats {
    steps: BTreeMap<String, u64>,
    current: Option<(String, Instant)>,
    times: BTreeMap<String, f64>,
}
impl Stats {
    fn report(&mut self, p: PreparationProgress) {
        if let Some(step) = p.step {
            if self.current.as_ref().is_none_or(|(id, _)| id != &step) {
                if let Some((id, at)) = self.current.take() {
                    *self.times.entry(id).or_default() += at.elapsed().as_secs_f64();
                }
                self.current = Some((step.clone(), Instant::now()));
            }
            let max = self.steps.entry(step).or_default();
            *max = (*max).max(p.completed);
        }
    }
    fn print(&self) {
        for suffix in ["hash", "decode", "resample", "headroom", "copy"] {
            let matching: Vec<_> = self
                .steps
                .iter()
                .filter(|(step, _)| step.ends_with(suffix))
                .collect();
            println!(
                "{suffix}_passes={} units={} seconds={:.3}",
                matching.len(),
                matching.iter().map(|(_, n)| **n).sum::<u64>(),
                self.times
                    .iter()
                    .filter(|(id, _)| id.ends_with(suffix))
                    .map(|(_, t)| *t)
                    .sum::<f64>()
            );
        }
        for (step, units) in &self.steps {
            println!("step={step} units={units}");
        }
    }
}
fn main() -> Result<()> {
    let seconds: u32 = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "15".into())
        .parse()?;
    ensure!(seconds > 0 && seconds <= 300, "Use 1–300 generated seconds");
    let fixture = fixture(seconds)?;
    let reference = fixture.0.join("reference.wav");
    let prototype = fixture.0.join("prototype");
    let owned = fixture.0.join(uuid::Uuid::new_v4().to_string());
    let mut stats = Stats::default();
    let start = Instant::now();
    let record = library::attach(&reference, &prototype, &owned, 48000, |p| stats.report(p))?;
    let attach = start.elapsed();
    let prepared = library::prepare(
        &record,
        &reference,
        &owned,
        &owned.join("playback-cache"),
        48000,
        |p| stats.report(p),
    )?;
    println!(
        "seconds={seconds} profile={} reference_sha={} attachment_s={:.3} cache_ready_s={:.3} total_s={:.3} prepared_bytes={}",
        if cfg!(debug_assertions) {
            "debug"
        } else {
            "release"
        },
        record.reference_sha256,
        attach.as_secs_f64(),
        start.elapsed().as_secs_f64() - attach.as_secs_f64(),
        start.elapsed().as_secs_f64(),
        prepared.temp_bytes
    );
    stats.print();
    drop(prepared);
    Ok(())
}
