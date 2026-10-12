mod identity;
mod records;
pub mod reset;
use crate::storage::atomic_write;
use crate::{audio::Audio, dsp::Controls, engine::LoopRegion};
use anyhow::{Context, Result, ensure};
pub use identity::*;
pub use records::*;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::HashSet,
    fs::{self, File},
    io::Read,
    path::{Path, PathBuf},
};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Section {
    pub id: String,
    pub name: String,
    pub start: f64,
    pub end: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Document {
    pub version: u32,
    pub song_key: String,
    pub reference_token: String,
    pub controls: Controls,
    pub sections: Vec<Section>,
    pub selected_section: Option<String>,
    pub loop_selection: Option<LoopRegion>,
    pub loop_enabled: bool,
    pub last_position: f64,
    #[serde(default)]
    pub notes: String,
    #[serde(default)]
    pub measure_map: Option<crate::timing::MeasureMap>,
}

impl Document {
    pub fn new(song_key: String) -> Self {
        Self {
            version: 5,
            song_key,
            reference_token: String::new(),
            controls: Controls::default(),
            sections: Vec::new(),
            selected_section: None,
            loop_selection: None,
            loop_enabled: false,
            last_position: 0.0,
            notes: String::new(),
            measure_map: None,
        }
    }

    pub fn validate(&self, key: &str, duration: f64) -> Result<()> {
        ensure!(
            self.version == 5,
            "Saved practice settings use an unsupported version"
        );
        ensure!(
            self.song_key == key,
            "Practice settings belong to a different song"
        );
        ensure!(
            self.notes.len() <= 1_048_576,
            "Song notes must be at most 1 MiB of UTF-8 text"
        );
        self.controls.validate()?;
        if let Some(map) = &self.measure_map {
            map.validate()?;
            ensure!(
                (map.frames as f64 / map.rate as f64 - duration).abs() < 1e-6,
                "Musical map belongs to a different source duration"
            );
        }
        ensure!(self.sections.len() <= 1000, "Too many sections");
        let mut ids = HashSet::new();
        for section in &self.sections {
            ensure!(
                !section.id.is_empty() && section.id.len() <= 128 && ids.insert(&section.id),
                "Section IDs must be unique"
            );
            ensure!(
                !section.name.trim().is_empty() && section.name.len() <= 128,
                "Section name must contain 1–128 characters"
            );
            LoopRegion {
                start: section.start,
                end: section.end,
                fade_ms: 0.0,
            }
            .validate(duration)?;
        }
        ensure!(
            self.selected_section
                .as_ref()
                .is_none_or(|id| ids.contains(id)),
            "Selected section no longer exists"
        );
        if let Some(region) = self.loop_selection {
            region.validate(duration)?;
        }
        ensure!(
            !self.loop_enabled || self.loop_selection.is_some(),
            "Choose a range before enabling a loop"
        );
        ensure!(
            self.last_position.is_finite() && (0.0..=duration).contains(&self.last_position),
            "Saved position is outside the song"
        );
        Ok(())
    }

    pub fn active_loop(&self) -> Option<LoopRegion> {
        self.loop_selection.filter(|_| self.loop_enabled)
    }
}

#[derive(Clone, Serialize)]
pub struct Song {
    pub key: String,
    pub asset_id: AudioAssetId,
    pub timeline_id: TimelineId,
    pub reference_revision: u64,
    pub session_id: SessionId,
    pub reference_token: String,
    pub retained_recordings: Vec<RecordingOption>,
    pub title: String,
    pub path: Option<String>,
    pub duration: f64,
    pub rate: u32,
    pub peaks: Vec<[f32; 2]>,
}

pub fn waveform(audio: &Audio) -> Vec<[f32; 2]> {
    let chunk = audio.frames.len().div_ceil(65_536).max(1);
    audio
        .frames
        .chunks(chunk)
        .map(|frames| {
            frames
                .iter()
                .flatten()
                .fold([0.0_f32, 0.0_f32], |[lo, hi], &s| [lo.min(s), hi.max(s)])
        })
        .collect()
}

/// Complete asset content digest (not song identity). Hashing stays off the audio worker.
pub fn asset_digest(path: &Path) -> Result<String> {
    let mut file = File::open(path)?;
    let mut hash = Sha256::new();
    let mut buffer = [0_u8; 65_536];
    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        hash.update(&buffer[..read]);
    }
    Ok(format!("{:x}", hash.finalize()))
}

pub struct Store {
    directory: PathBuf,
    reset_notice: bool,
}
impl Store {
    pub fn new(directory: PathBuf) -> Result<Self> {
        fs::create_dir_all(&directory).context("Cannot create the practice settings folder")?;
        let reset_notice = reset::reset_legacy(&directory)?;
        Ok(Self {
            directory,
            reset_notice,
        })
    }
    pub fn reset_notice(&self) -> bool {
        self.reset_notice
    }
    fn document_path(&self, key: &str) -> Result<PathBuf> {
        let id: SongId = key.to_owned().try_into()?;
        Ok(self.directory.join(format!("{id}.json")))
    }
    pub fn record(&self, key: &str) -> Result<SongRecord> {
        let file = File::open(self.document_path(key)?)?;
        ensure!(
            file.metadata()?.len() <= 16 * 1024 * 1024,
            "Song record exceeds 16 MiB"
        );
        let record: SongRecord = serde_json::from_reader(file)
            .context("Cannot read song record; original file preserved")?;
        record.validate()?;
        ensure!(
            record.id.to_string() == key,
            "Song record identity mismatch"
        );
        Ok(record)
    }
    pub fn save_record(&self, record: &SongRecord) -> Result<()> {
        record.validate()?;
        let bytes = serde_json::to_vec_pretty(record)?;
        ensure!(
            bytes.len() <= 16 * 1024 * 1024,
            "Song record exceeds 16 MiB"
        );
        let path = self.document_path(&record.id.to_string())?;
        match fs::symlink_metadata(&path) {
            Ok(metadata) => {
                ensure!(
                    metadata.is_file() && !metadata.file_type().is_symlink(),
                    "Song record must be a regular file; original preserved"
                );
                self.record(&record.id.to_string())?;
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(error.into()),
        }
        atomic_write(&path, &bytes)
    }
    pub fn load(&self, key: &str, duration: f64) -> Result<Document> {
        let record = self.record(key)?;
        let document = record.document(SessionId::new())?;
        document.validate(key, duration)?;
        Ok(document)
    }
    pub fn contains(&self, key: &str) -> Result<bool> {
        Ok(self.document_path(key)?.exists())
    }
    pub fn save(&self, document: &Document, duration: f64) -> Result<()> {
        document.validate(&document.song_key, duration)?;
        let mut record = self.record(&document.song_key)?;
        record.update_document(document)?;
        self.save_record(&record)
    }
    pub fn last_song(&self) -> Result<Option<String>> {
        let path = self.directory.join("last-song.json");
        match fs::symlink_metadata(&path) {
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(error.into()),
            Ok(metadata) => ensure!(
                metadata.is_file() && !metadata.file_type().is_symlink() && metadata.len() <= 4096,
                "Last-song record must be a regular file at most 4 KiB; original preserved"
            ),
        }
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Last {
            version: u32,
            song_id: SongId,
        }
        let last: Last = serde_json::from_slice(&fs::read(path)?)?;
        ensure!(
            last.version == 2,
            "Unsupported last-song schema; original file preserved"
        );
        Ok(Some(last.song_id.to_string()))
    }
    pub fn remember(&self, key: &str) -> Result<()> {
        let id: SongId = key.to_owned().try_into()?;
        self.last_song()?; // Never overwrite unsupported or unreadable restoration state.
        atomic_write(
            &self.directory.join("last-song.json"),
            &serde_json::to_vec(&serde_json::json!({"version":2,"song_id":id}))?,
        )
    }
}
