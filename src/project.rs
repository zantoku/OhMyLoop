use crate::storage::atomic_write;
use crate::{audio::Audio, dsp::Controls, engine::LoopRegion};
use anyhow::{Context, Result, ensure};
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
            version: 4,
            song_key,
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
            self.version == 4,
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

/// Content identity survives moving or renaming a file, and rejects replacement
/// audio at the same path. Hashing happens off the audio worker.
pub fn song_key(path: &Path) -> Result<String> {
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
}

impl Store {
    pub fn new(directory: PathBuf) -> Result<Self> {
        fs::create_dir_all(&directory).context("Cannot create the practice settings folder")?;
        Ok(Self { directory })
    }
    fn document_path(&self, key: &str) -> Result<PathBuf> {
        ensure!(
            key.len() <= 128
                && !key.is_empty()
                && key.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'-'),
            "Invalid song identity"
        );
        Ok(self.directory.join(format!("{key}.json")))
    }
    pub fn load(&self, key: &str, duration: f64) -> Result<Document> {
        let path = self.document_path(key)?;
        if !path.exists() {
            return Ok(Document::new(key.into()));
        }
        let mut document: Document = serde_json::from_slice(&fs::read(&path)?).context(
            "Saved practice settings could not be read; the original file was left intact",
        )?;
        if document.version == 2 {
            ensure!(
                document.measure_map.is_none(),
                "Version-2 settings cannot contain a musical map"
            );
            document.version = 3;
        }
        if document.version == 3 {
            document.version = 4;
        }
        document.validate(key, duration)?;
        Ok(document)
    }
    pub fn contains(&self, key: &str) -> Result<bool> {
        Ok(self.document_path(key)?.exists())
    }
    pub fn save(&self, document: &Document, duration: f64) -> Result<()> {
        document.validate(&document.song_key, duration)?;
        atomic_write(
            &self.document_path(&document.song_key)?,
            &serde_json::to_vec_pretty(document)?,
        )
    }
    pub fn last_song(&self) -> Result<Option<String>> {
        let path = self.directory.join("last-song.json");
        if !path.exists() {
            return Ok(None);
        }
        Ok(serde_json::from_slice(&fs::read(path)?)?)
    }
    pub fn remember(&self, source: &str) -> Result<()> {
        atomic_write(
            &self.directory.join("last-song.json"),
            &serde_json::to_vec(source)?,
        )
    }
}
