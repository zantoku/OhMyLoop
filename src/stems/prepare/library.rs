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

/// Validates the prototype contract, copies only declared stem audio, then validates
/// owned copies. Caller publishes the association before deleting an older set.
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
    let original_manifest = fs::read(&manifest_path)?;
    let original_value: serde_json::Value = serde_json::from_slice(&original_manifest)?;
    supported_provenance(&original_value["identity"])?;
    ensure!(
        original_value["corrections"]
            .as_array()
            .is_some_and(|corrections| corrections.is_empty()),
        "Unsupported or missing alignment corrections"
    );
    let validated = PreparedComparison::prepare(
        reference,
        &[prototype.to_path_buf()],
        output_rate,
        &mut progress,
    )?;
    drop(validated);
    let bytes = fs::read(safe_file(prototype, "manifest.json")?)?;
    ensure!(
        bytes == original_manifest,
        "Prototype manifest changed during validation"
    );
    let manifest: Manifest = serde_json::from_slice(&bytes)?;
    let raw: serde_json::Value = serde_json::from_slice(&bytes)?;
    directory_safe(owned)?;
    ensure!(!owned.exists(), "Stem destination already exists");
    fs::create_dir_all(owned)?;
    let result = (|| {
        let mut validation = Preparer {
            directory: std::env::temp_dir().join(format!(
                "ohmyloop-owned-validation-{}",
                uuid::Uuid::new_v4()
            )),
            next: 0,
            progress: &mut progress,
            step: None,
        };
        fs::create_dir(&validation.directory)?;
        let _temporary = TemporaryDirectory(validation.directory.clone(), false.into());
        let preparation = &manifest.identity.preparation;
        let mut tracks = Vec::new();
        let mut source_files = std::collections::HashSet::new();
        for (role, track) in &manifest.stems {
            ensure!(
                source_files.insert(&track.file),
                "Duplicate declared stem file"
            );
            let source = safe_file(prototype, &track.file)?;
            // Generated filenames avoid carrying input directory structures into storage.
            let extension = source.extension().and_then(|e| e.to_str()).unwrap_or("wav");
            let file = format!("{role}.{extension}");
            let destination = owned.join(&file);
            validation.stage(format!("owned.{role}.copy"));
            let total = fs::metadata(&source)?.len();
            let mut input = File::open(&source)?;
            let mut output = File::create(&destination)?;
            let mut buffer = vec![0; 256 * 1024];
            let mut copied = 0;
            loop {
                let n = input.read(&mut buffer)?;
                if n == 0 {
                    break;
                }
                output.write_all(&buffer[..n])?;
                copied += n as u64;
                validation.notify(format!("Copy {role}"), copied, Some(total));
            }
            output.sync_all()?;
            validation.stage(format!("owned.{role}.hash"));
            ensure!(
                validation.digest(&destination)? == track.sha256,
                "Owned stem digest mismatch"
            );
            validation.stage(format!("owned.{role}.decode"));
            let pcm = validation.decode(&destination)?;
            ensure!(
                pcm.rate == track.rate
                    && pcm.frames == track.frames
                    && pcm.channels == track.channels,
                "Owned stem geometry mismatch"
            );
            tracks.push(StoredTrack {
                role: role.clone(),
                file,
                sha256: track.sha256.clone(),
                rate: track.rate,
                frames: track.frames,
                channels: track.channels,
            });
        }
        let record = StoredStemSet {
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
            provenance: raw["identity"].clone(),
            tracks,
            mix: SavedMix::default(),
        };
        record.validate()?;
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
