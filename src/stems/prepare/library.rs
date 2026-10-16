//! App-owned stem records and rebuildable current-device preparation cache.
use super::*;
use serde::{Deserialize, Serialize};
mod cache;
pub use super::{WorkflowProgress, WorkflowStep, WorkflowTracker, throttled_progress};
pub use cache::prepare;

const VERSION: u32 = 1;
// Includes sinc kernel/window/block configuration and the shared 0.70 headroom policy.
const POLICY: u32 = 2;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct SavedTrack {
    pub gain: f32,
    pub mute: bool,
}
impl Default for SavedTrack {
    fn default() -> Self {
        Self {
            gain: 1.0,
            mute: false,
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Default)]
#[serde(deny_unknown_fields)]
pub struct SavedMix {
    pub tracks: [SavedTrack; 6],
}
impl SavedMix {
    pub fn to_mix(&self) -> MixState {
        MixState {
            tracks: std::array::from_fn(|i| super::super::RoleControl {
                gain: self.tracks[i].gain,
                mute: self.tracks[i].mute,
                solo: false,
            }),
        }
    }
    pub fn from_mix(mix: MixState) -> Self {
        Self {
            tracks: std::array::from_fn(|i| SavedTrack {
                gain: mix.tracks[i].gain,
                mute: mix.tracks[i].mute,
            }),
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct StoredTrack {
    pub role: String,
    pub file: String,
    pub sha256: String,
    pub rate: u32,
    pub frames: u64,
    pub channels: u32,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct StoredStemSet {
    pub version: u32,
    pub id: String,
    pub reference_sha256: String,
    pub reference_rate: u32,
    pub reference_frames: u64,
    pub provenance: serde_json::Value,
    pub tracks: Vec<StoredTrack>,
    #[serde(default)]
    pub mix: SavedMix,
}
const PREPROCESSING: &str = "symphonia-stereo-rubato-delay-compensated-v1";
const GEOMETRY_POLICY: &str =
    "floor(seconds * source_rate); floor(span_frames * 44100 / source_rate); no context";

fn supported_provenance(identity: &serde_json::Value) -> Result<()> {
    let preparation = &identity["preparation"];
    ensure!(
        preparation["preprocessing"].as_str() == Some(PREPROCESSING)
            && preparation["geometry_policy"].as_str() == Some(GEOMETRY_POLICY)
            && preparation["prepared_rate"].as_u64() == Some(44_100),
        "Unsupported prototype preprocessing or coordinate policy"
    );
    let lock = &identity["model_lock"];
    ensure!(
        lock["version"].as_u64() == Some(1),
        "Missing or unsupported model identity"
    );
    let expected = match lock["model"].as_str() {
        Some("demucs-6s") => ["5c90dfd2-34c22ccb.th", "htdemucs_6s.yaml"],
        Some("roformer-sw") => ["BS-Roformer-SW.ckpt", "BS-Roformer-SW.yaml"],
        _ => anyhow::bail!("Unsupported separation model identity"),
    };
    ensure!(
        identity["settings"]["normalization"].as_str() == Some("none per stem"),
        "Unsupported per-stem normalization policy"
    );
    if lock["model"].as_str() == Some("roformer-sw") {
        ensure!(
            identity["settings"]["roformer_short_input"].as_str()
                == Some("right-zero-pad-to-config-chunk; crop-declared-padding"),
            "Unsupported Roformer padding policy"
        );
    }
    let files = lock["files"]
        .as_object()
        .context("Missing model artifacts")?;
    ensure!(
        files.len() == expected.len(),
        "Incomplete model artifact identity"
    );
    for name in expected {
        let file = files.get(name).context("Missing model artifact identity")?;
        let hash = file["sha256"]
            .as_str()
            .context("Missing model artifact digest")?;
        ensure!(
            digest_valid(hash)
                && file["bytes"].as_u64().is_some_and(|n| n > 0)
                && file["url"]
                    .as_str()
                    .is_some_and(|url| url.starts_with("https://")),
            "Invalid model artifact identity"
        );
        if name.ends_with(".ckpt") {
            ensure!(
                hash == "24e7d35ee9c64415673d3fd33e06a67cac2c103c5df6267ba1576459c775916e",
                "Unknown Roformer checkpoint"
            );
        } else if name.ends_with(".th") {
            ensure!(hash.starts_with("34c22ccb"), "Unknown Demucs checkpoint");
        }
    }
    Ok(())
}
fn digest_valid(hash: &str) -> bool {
    hash.len() == 64 && hash.bytes().all(|b| b.is_ascii_hexdigit())
}
fn relative_valid(file: &str) -> bool {
    !file.is_empty()
        && !file.contains('\\')
        && Path::new(file)
            .components()
            .all(|c| matches!(c, Component::Normal(_)))
}
impl StoredStemSet {
    pub fn validate(&self) -> Result<()> {
        ensure!(self.version == VERSION, "Unsupported stem record version");
        uuid::Uuid::parse_str(&self.id).context("Invalid stem set identity")?;
        ensure!(
            digest_valid(&self.reference_sha256),
            "Invalid reference digest"
        );
        ensure!(
            (8_000..=192_000).contains(&self.reference_rate) && self.reference_frames > 0,
            "Invalid reference geometry"
        );
        ensure!(
            !self.tracks.is_empty() && self.tracks.len() <= 6,
            "Invalid stem role count"
        );
        let mut roles = std::collections::HashSet::new();
        let mut files = std::collections::HashSet::new();
        for track in &self.tracks {
            Role::parse(&track.role)?;
            ensure!(
                roles.insert(&track.role) && files.insert(&track.file),
                "Duplicate stem role or file"
            );
            ensure!(
                relative_valid(&track.file) && digest_valid(&track.sha256),
                "Invalid stored stem path or digest"
            );
            ensure!(
                (8_000..=192_000).contains(&track.rate)
                    && track.channels == 2
                    && track.frames
                        == coordinate_frames(
                            self.reference_frames,
                            track.rate,
                            self.reference_rate
                        )?,
                "Invalid stem geometry"
            );
        }
        supported_provenance(&self.provenance)?;
        if let Some(corrections) = self.provenance.get("alignment_corrections") {
            validate_corrections(
                &serde_json::json!({"identity":self.provenance,"corrections":corrections}),
            )?;
        }
        let identity: Identity = serde_json::from_value(self.provenance.clone())
            .context("Invalid verified provenance")?;
        let preparation = identity.preparation;
        ensure!(
            preparation.version == 1
                && preparation.channels == 2
                && preparation.source_sha256 == self.reference_sha256
                && preparation.source_rate == self.reference_rate
                && preparation.source_frames == self.reference_frames
                && preparation.source_span.start == 0
                && preparation.source_span.end_exclusive == self.reference_frames
                && (8_000..=192_000).contains(&preparation.prepared_rate)
                && preparation.prepared_frames
                    == coordinate_frames(
                        self.reference_frames,
                        preparation.prepared_rate,
                        self.reference_rate
                    )?
                && digest_valid(&preparation.prepared_sha256),
            "Invalid verified provenance mapping"
        );
        ensure!(
            self.tracks
                .iter()
                .all(|t| t.rate == preparation.prepared_rate
                    && t.frames == preparation.prepared_frames),
            "Stored tracks disagree with provenance"
        );
        self.mix.to_mix().validate()
    }
    pub fn inherit_saved_mix(&mut self, previous: Option<&Self>) {
        if let Some(previous) = previous {
            for role in self.roles() {
                if previous.roles().contains(&role) {
                    self.mix.tracks[role.index()] = previous.mix.tracks[role.index()].clone();
                }
            }
        }
    }
    pub fn roles(&self) -> Vec<Role> {
        self.tracks
            .iter()
            .filter_map(|t| Role::parse(&t.role).ok())
            .collect()
    }
    /// Only present roles participate in automatic Original/Stems selection.
    pub fn selection(&self, mix: MixState) -> usize {
        usize::from(self.roles().iter().any(|r| {
            let control = mix.tracks[r.index()];
            control.gain != 1.0 || control.mute || control.solo
        }))
    }
}

fn verified_provenance(raw: &serde_json::Value) -> Result<serde_json::Value> {
    validate_corrections(raw)?;
    let mut value = raw["identity"].clone();
    value
        .as_object_mut()
        .context("Invalid provenance identity")?
        .insert("alignment_corrections".into(), raw["corrections"].clone());
    Ok(value)
}
fn directory_safe(path: &Path) -> Result<()> {
    let mut cursor = PathBuf::new();
    for part in path.components() {
        cursor.push(part);
        ensure!(
            !matches!(part, Component::ParentDir),
            "Owned paths must not traverse parents"
        );
        match fs::symlink_metadata(&cursor) {
            Ok(metadata) => ensure!(
                !metadata.file_type().is_symlink(),
                "Owned paths must not contain symlinks"
            ),
            Err(error) if error.kind() == ErrorKind::NotFound => (),
            Err(error) => return Err(error.into()),
        }
    }
    Ok(())
}

/// Copy declared stems, validate the owned bytes once, then retain their bounded
/// PCM/headroom as a rebuildable cache. Publication remains the caller's job.
pub fn attach(
    reference: &Path,
    prototype: &Path,
    owned: &Path,
    output_rate: u32,
    mut progress: impl FnMut(PreparationProgress),
) -> Result<StoredStemSet> {
    directory_safe(prototype)?;
    let manifest_path = safe_file(prototype, "manifest.json")?;
    ensure!(
        fs::metadata(&manifest_path)?.len() <= 4 * 1024 * 1024,
        "Oversized prototype manifest"
    );
    let original_manifest = crate::storage::read_limited(&manifest_path, 4 * 1024 * 1024)?;
    let raw: serde_json::Value = serde_json::from_slice(&original_manifest)?;
    supported_provenance(&raw["identity"])?;
    validate_corrections(&raw)?;
    let manifest: Manifest = serde_json::from_slice(&original_manifest)?;
    ensure!(
        manifest.version == 1 && manifest.status == "complete",
        "Unsupported or incomplete prototype manifest"
    );
    ensure!(
        format!(
            "{:x}",
            Sha256::digest(serde_json::to_vec(&raw["identity"])?)
        ) == manifest.job_key,
        "Invalid manifest job identity"
    );
    let mut source_files = std::collections::HashSet::new();
    let mut sources = Vec::new();
    let mut tracks = Vec::new();
    let mut paths = BTreeMap::new();
    for (role, track) in &manifest.stems {
        Role::parse(role)?;
        ensure!(
            source_files.insert(&track.file),
            "Duplicate declared stem file"
        );
        let source = safe_file(prototype, &track.file)?;
        let extension = source.extension().and_then(|e| e.to_str()).unwrap_or("wav");
        let file = format!("{role}.{extension}");
        tracks.push(StoredTrack {
            role: role.clone(),
            file: file.clone(),
            sha256: track.sha256.clone(),
            rate: track.rate,
            frames: track.frames,
            channels: track.channels,
        });
        paths.insert(role.clone(), owned.join(&file));
        sources.push((role.clone(), source, file));
    }
    let preparation = &manifest.identity.preparation;
    let mut record = StoredStemSet {
        version: VERSION,
        id: owned
            .file_name()
            .and_then(|n| n.to_str())
            .and_then(|n| uuid::Uuid::parse_str(n).ok())
            .unwrap_or_else(uuid::Uuid::new_v4)
            .to_string(),
        reference_sha256: preparation.source_sha256.clone(),
        reference_rate: preparation.source_rate,
        reference_frames: preparation.source_frames,
        provenance: verified_provenance(&raw)?,
        tracks,
        mix: SavedMix::default(),
    };
    record.validate()?;
    directory_safe(owned)?;
    ensure!(!owned.exists(), "Stem destination already exists");
    fs::create_dir_all(owned)?;
    let result = (|| {
        let mut buffer = vec![0; 256 * 1024];
        for (role, source, file) in sources {
            let mut input = File::open(&source)?;
            let total = input.metadata()?.len();
            let mut output = fs::OpenOptions::new()
                .create_new(true)
                .write(true)
                .open(owned.join(file))?;
            let mut copied = 0;
            let step = format!("owned.{role}.copy");
            progress(PreparationProgress {
                phase: format!("Copy {role}"),
                completed: 0,
                total: Some(total),
                step: Some(step.clone()),
                workflow: None,
            });
            loop {
                let n = input.read(&mut buffer)?;
                if n == 0 {
                    break;
                }
                output.write_all(&buffer[..n])?;
                copied += n as u64;
                progress(PreparationProgress {
                    phase: format!("Copy {role}"),
                    completed: copied,
                    total: Some(total),
                    step: Some(step.clone()),
                    workflow: None,
                });
            }
            ensure!(copied == total, "Stem changed while copying: {role}");
            output.sync_all()?;
        }
        // Metadata is captured before hash/decode, never refreshed after validation
        // and presented as evidence that older PCM represents newer input bytes.
        let checked = cache::capture_inputs(&record, reference, owned)?;
        let evidence_path = safe_file(prototype, "prepared.wav")?;
        let evidence = file_state(&evidence_path)?;
        let (validated, excluded) = PreparedComparison::prepare_owned(
            reference,
            prototype,
            &paths,
            output_rate,
            &mut progress,
        )?;
        ensure!(
            crate::storage::read_limited(&safe_file(prototype, "manifest.json")?, 4 * 1024 * 1024)?
                == original_manifest,
            "Prototype manifest changed during validation"
        );
        ensure!(
            file_state(&safe_file(prototype, "prepared.wav")?)? == evidence,
            "Prepared input changed during validation"
        );
        cache::verify_inputs(&record, reference, owned, &checked)?;
        let original_record = record.clone();
        record
            .tracks
            .retain(|track| !excluded.contains(&track.role));
        let checked = checked.retained(&original_record, &record)?;
        for track in original_record
            .tracks
            .iter()
            .filter(|track| excluded.contains(&track.role))
        {
            fs::remove_file(safe_file(owned, &track.file)?)?;
        }
        let adopted = cache::adopt(
            &record,
            reference,
            owned,
            &owned.join("playback-cache"),
            validated,
            &checked,
            &mut progress,
        )?;
        drop(adopted);
        Ok(record)
    })();
    match result {
        Err(error) => match fs::remove_dir_all(owned) {
            Ok(()) => Err(error),
            Err(cleanup) => {
                Err(error.context(format!("Attachment cleanup also failed: {cleanup}")))
            }
        },
        success => success,
    }
}

fn file_state(path: &Path) -> Result<(u64, std::time::SystemTime)> {
    let metadata = fs::metadata(path)?;
    ensure!(metadata.is_file(), "Expected an audio file");
    Ok((metadata.len(), metadata.modified()?))
}

/// Only the pinned adapter's exact known short-input geometry is accepted.
pub fn validate_corrections(raw: &serde_json::Value) -> Result<()> {
    let frames = raw["identity"]["preparation"]["prepared_frames"]
        .as_u64()
        .context("Missing prepared frames")?;
    let roformer = raw["identity"]["model_lock"]["model"].as_str() == Some("roformer-sw");
    let actual = raw["corrections"]
        .as_array()
        .context("Missing alignment corrections")?;
    if roformer && frames > 0 && frames < 409600 {
        ensure!(
            raw["identity"]["model_lock"]["files"]["BS-Roformer-SW.yaml"]["sha256"].as_str()
                == Some("b558996f1e25eb48798bd6502505a5de94c4f966d6edfb1a0420f06cc40b501a"),
            "Short-input correction requires the pinned Roformer configuration"
        );
    }
    let expected = if roformer && frames > 0 && frames < 409600 {
        vec![
            serde_json::json!({"kind":"roformer-short-input-right-zero-pad-and-crop","input_frames":frames,"padded_frames":409600,"right_padding_frames":409600-frames,"crop_start":0,"crop_end_exclusive":frames}),
        ]
    } else {
        vec![]
    };
    ensure!(
        *actual == expected,
        "Unsupported or contradictory alignment corrections"
    );
    Ok(())
}
/// Validate the generated bytes in their final private owned directory. No WAV copy.
pub fn attach_generated(
    reference: &Path,
    owned: &Path,
    output_rate: u32,
    control: &super::super::WorkControl,
    mut progress: impl FnMut(PreparationProgress),
) -> Result<(StoredStemSet, PreparedComparison)> {
    control.check()?;
    directory_safe(owned)?;
    let manifest_path = safe_file(owned, "manifest.json")?;
    ensure!(
        fs::metadata(&manifest_path)?.len() <= 1024 * 1024,
        "Oversized generated manifest"
    );
    let bytes = crate::storage::read_limited(&manifest_path, 1024 * 1024)?;
    let raw: serde_json::Value = serde_json::from_slice(&bytes)?;
    supported_provenance(&raw["identity"])?;
    validate_corrections(&raw)?;
    let manifest: Manifest = serde_json::from_slice(&bytes)?;
    ensure!(
        manifest.version == 1 && manifest.status == "complete" && manifest.stems.len() == 6,
        "Incomplete generated output"
    );
    ensure!(
        format!(
            "{:x}",
            Sha256::digest(serde_json::to_vec(&raw["identity"])?)
        ) == manifest.job_key,
        "Invalid generated manifest identity"
    );
    let prep = &manifest.identity.preparation;
    let mut paths = BTreeMap::new();
    let mut tracks = Vec::new();
    for (role, track) in manifest.stems {
        Role::parse(&role)?;
        ensure!(
            track.file == format!("{role}.wav"),
            "Unexpected generated filename"
        );
        paths.insert(role.clone(), safe_file(owned, &track.file)?);
        tracks.push(StoredTrack {
            role,
            file: track.file,
            sha256: track.sha256,
            rate: track.rate,
            frames: track.frames,
            channels: track.channels,
        });
    }
    let mut record = StoredStemSet {
        version: VERSION,
        id: owned
            .file_name()
            .context("Missing set identity")?
            .to_string_lossy()
            .into_owned(),
        reference_sha256: prep.source_sha256.clone(),
        reference_rate: prep.source_rate,
        reference_frames: prep.source_frames,
        provenance: verified_provenance(&raw)?,
        tracks,
        mix: SavedMix::default(),
    };
    record.validate()?;
    let checked = cache::capture_inputs(&record, reference, owned)?;
    let evidence = file_state(&safe_file(owned, "prepared.wav")?)?;
    let (validated, excluded) = PreparedComparison::prepare_owned_control(
        reference,
        owned,
        &paths,
        output_rate,
        control,
        &mut progress,
    )?;
    control.check()?;
    ensure!(
        crate::storage::read_limited(&manifest_path, 1024 * 1024)? == bytes
            && file_state(&safe_file(owned, "prepared.wav")?)? == evidence,
        "Generated provenance changed during validation"
    );
    cache::verify_inputs(&record, reference, owned, &checked)?;
    let original = record.clone();
    record.tracks.retain(|t| !excluded.contains(&t.role));
    let checked = checked.retained(&original, &record)?;
    for track in original
        .tracks
        .iter()
        .filter(|t| excluded.contains(&t.role))
    {
        control.check()?;
        fs::remove_file(safe_file(owned, &track.file)?)?;
    }
    let prepared = cache::adopt_control(
        &record,
        reference,
        owned,
        validated,
        &checked,
        control,
        &mut progress,
    )?;
    control.check()?;
    Ok((record, prepared))
}
#[cfg(test)]
mod correction_tests {
    use super::*;
    #[test]
    fn known_short_padding_is_exact_and_normal_inputs_have_none() {
        let mut raw = serde_json::json!({"identity":{"model_lock":{"model":"roformer-sw","files":{"BS-Roformer-SW.yaml":{"sha256":"b558996f1e25eb48798bd6502505a5de94c4f966d6edfb1a0420f06cc40b501a"}}},"preparation":{"prepared_frames":44100}},"corrections":[{"kind":"roformer-short-input-right-zero-pad-and-crop","input_frames":44100,"padded_frames":409600,"right_padding_frames":365500,"crop_start":0,"crop_end_exclusive":44100}]});
        validate_corrections(&raw).unwrap();
        let mut wrong_config = raw.clone();
        wrong_config["identity"]["model_lock"]["files"]["BS-Roformer-SW.yaml"]["sha256"] =
            serde_json::json!("0".repeat(64));
        assert!(validate_corrections(&wrong_config).is_err());
        for field in [
            "input_frames",
            "padded_frames",
            "right_padding_frames",
            "crop_start",
            "crop_end_exclusive",
        ] {
            let mut bad = raw.clone();
            bad["corrections"][0][field] = serde_json::json!(1);
            assert!(validate_corrections(&bad).is_err());
        }
        raw["corrections"] = serde_json::json!([]);
        assert!(validate_corrections(&raw).is_err());
        raw["identity"]["preparation"]["prepared_frames"] = serde_json::json!(409600);
        validate_corrections(&raw).unwrap();
    }
}
