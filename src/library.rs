//! Library metadata and durable catalogue, separate from practice annotations.
mod tags;
use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashSet,
    fs,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};
pub use tags::read_tags;

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Metadata {
    pub artist: String,
    pub title: String,
    pub album: String,
}
impl Metadata {
    pub(crate) fn normalized(mut self) -> Result<Self> {
        self.artist = self.artist.trim().to_owned();
        self.title = self.title.trim().to_owned();
        self.album = self.album.trim().to_owned();
        ensure!(!self.title.is_empty(), "Song title cannot be empty");
        ensure!(
            [&self.artist, &self.title, &self.album]
                .iter()
                .all(|s| s.len() <= 1024 && !s.contains('\0')),
            "Song metadata must be at most 1024 bytes per field and contain no null characters"
        );
        Ok(self)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Entry {
    pub song_key: String,
    pub metadata: Metadata,
    pub file_location: String,
    /// UTC milliseconds since Unix epoch; displayed in the user's local time.
    pub date_added: u64,
    pub last_opened: u64,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Catalogue {
    pub version: u32,
    pub songs: Vec<Entry>,
}
impl Default for Catalogue {
    fn default() -> Self {
        Self {
            version: 2,
            songs: vec![],
        }
    }
}
impl Catalogue {
    pub(crate) fn validate(&self) -> Result<()> {
        ensure!(
            self.version == 2,
            "Unsupported library version; the original file was left intact"
        );
        ensure!(self.songs.len() <= 10_000, "Library exceeds 10,000 songs");
        let mut keys = HashSet::new();
        for entry in &self.songs {
            ensure!(
                crate::project::SongId::try_from(entry.song_key.clone()).is_ok()
                    && keys.insert(&entry.song_key),
                "Invalid or duplicate library song identity"
            );
            ensure!(
                !entry.file_location.is_empty()
                    && entry.file_location.len() <= 16_384
                    && !entry.file_location.contains('\0'),
                "Invalid library file location"
            );
            ensure!(
                entry.date_added <= entry.last_opened && entry.last_opened <= 8_640_000_000_000_000,
                "Invalid library dates"
            );
            ensure!(
                entry.metadata.clone().normalized()? == entry.metadata,
                "Library metadata contains surrounding whitespace"
            );
        }
        Ok(())
    }
    pub fn entry(&self, key: &str) -> Result<&Entry> {
        self.songs
            .iter()
            .find(|s| s.song_key == key)
            .context("Song is not in the library")
    }
}

/// A song in the list the UI shows: the persisted catalogue entry plus facts
/// read from the song's recording at read time. Never written to disk, so
/// catalogue files stay readable by older builds.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct LibraryRow {
    #[serde(flatten)]
    pub entry: Entry,
    /// Length of the recording in milliseconds; 0 when it can't be read.
    pub duration_ms: u64,
}
/// The song list as the UI reads it; see [`LibraryRow`].
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct LibraryView {
    pub version: u32,
    pub songs: Vec<LibraryRow>,
}
impl LibraryView {
    pub fn new(catalogue: Catalogue, duration_ms: impl Fn(&str) -> u64) -> Self {
        Self {
            version: catalogue.version,
            songs: catalogue
                .songs
                .into_iter()
                .map(|entry| LibraryRow {
                    duration_ms: duration_ms(&entry.song_key),
                    entry,
                })
                .collect(),
        }
    }
    pub fn entry(&self, key: &str) -> Result<&Entry> {
        self.songs
            .iter()
            .map(|row| &row.entry)
            .find(|s| s.song_key == key)
            .context("Song is not in the library")
    }
}

/// Audio files that can be imported, by extension.
pub const SONG_EXTENSIONS: [&str; 8] = ["wav", "mp3", "flac", "ogg", "m4a", "aac", "aiff", "aif"];
/// Rejects a file that isn't a supported recording before it is hashed or copied.
pub fn ensure_importable(path: &Path) -> Result<()> {
    let supported = path
        .extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| {
            SONG_EXTENSIONS
                .iter()
                .any(|known| extension.eq_ignore_ascii_case(known))
        });
    ensure!(
        supported,
        "This file can't be imported. Choose a WAV, MP3, FLAC, Ogg, M4A, AAC or AIFF recording."
    );
    Ok(())
}

pub struct Store {
    path: PathBuf,
}
impl Store {
    // Opening a corrupt index must not prevent the existing practice workflow.
    pub fn new(directory: &Path) -> Self {
        Self {
            path: directory.join("library.json"),
        }
    }
    pub fn read(&self) -> Result<Catalogue> {
        let file = match fs::File::open(&self.path) {
            Ok(file) => file,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Catalogue::default()),
            Err(e) => {
                return Err(e)
                    .context("Cannot read song library; the original file was left intact");
            }
        };
        ensure!(
            file.metadata()?.len() <= 16 * 1024 * 1024,
            "Library file exceeds 16 MiB; the original file was left intact"
        );
        let catalogue: Catalogue = serde_json::from_reader(file)
            .context("Cannot read song library; the original file was left intact")?;
        catalogue.validate()?;
        Ok(catalogue)
    }
    fn save(&self, catalogue: &Catalogue) -> Result<()> {
        catalogue.validate()?;
        let bytes = serde_json::to_vec_pretty(catalogue)?;
        ensure!(
            bytes.len() <= 16 * 1024 * 1024,
            "Library exceeds its 16 MiB storage limit"
        );
        crate::storage::atomic_write(&self.path, &bytes)
    }
    pub fn opened(
        &self,
        key: String,
        location: String,
        metadata: Metadata,
        now: u64,
    ) -> Result<Entry> {
        let mut catalogue = self.read()?;
        let entry = if let Some(entry) = catalogue.songs.iter_mut().find(|s| s.song_key == key) {
            // User edits take precedence over tags on every subsequent open.
            entry.file_location = location;
            entry.last_opened = now.max(entry.last_opened);
            entry.clone()
        } else {
            let entry = Entry {
                song_key: key,
                metadata: metadata.normalized()?,
                file_location: location,
                date_added: now,
                last_opened: now,
            };
            catalogue.songs.push(entry.clone());
            entry
        };
        self.save(&catalogue)?;
        Ok(entry)
    }
    pub fn update(&self, key: &str, metadata: Metadata) -> Result<Catalogue> {
        let mut catalogue = self.read()?;
        let entry = catalogue
            .songs
            .iter_mut()
            .find(|s| s.song_key == key)
            .context("Song is not in the library")?;
        entry.metadata = metadata.normalized()?;
        self.save(&catalogue)?;
        Ok(catalogue)
    }
}
pub fn now_ms() -> Result<u64> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .context("System clock predates Unix epoch")?
        .as_millis()
        .try_into()
        .context("System clock exceeds supported dates")
}
