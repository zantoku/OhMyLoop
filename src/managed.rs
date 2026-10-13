//! User-owned managed recording storage. Multi-file operations are not transactional.
mod paths;
mod preferences;
mod transfers;
use crate::{
    library::{Catalogue, Entry, Metadata, now_ms},
    project::{LibraryId, SongRecord},
};
use anyhow::{Context, Result, ensure};
pub use preferences::{LibraryPointer, PreferenceData, Preferences};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::{Path, PathBuf},
};
pub use transfers::{Prepared, PreparedReplacement};

#[derive(Clone, Debug)]
pub struct ManagedLibrary {
    root: PathBuf,
    id: LibraryId,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Manifest {
    version: u32,
    id: LibraryId,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ManagedSong {
    pub version: u32,
    pub record: SongRecord,
    pub metadata: Metadata,
    pub date_added: u64,
    pub last_opened: u64,
}
impl ManagedLibrary {
    pub fn create(parent: &Path) -> Result<Self> {
        let root = parent.join("OhMyLoop Library");
        fs::create_dir(&root).context(
            "Cannot create library; choose a parent without an existing OhMyLoop Library folder",
        )?;
        let library = Self {
            root: root.canonicalize()?,
            id: LibraryId::new(),
        };
        fs::create_dir(library.root.join("songs"))?;
        library.write(
            "library.json",
            &Manifest {
                version: 1,
                id: library.id,
            },
        )?;
        library.write("catalogue.json", &Catalogue::default())?;
        Ok(library)
    }
    pub fn open(root: &Path) -> Result<Self> {
        ensure!(root.is_dir(), "Library folder is unavailable");
        let mut library = Self {
            root: root.canonicalize()?,
            id: LibraryId::new(),
        };
        let manifest: Manifest = library.read("library.json")?;
        ensure!(
            manifest.version == 1,
            "Unsupported library schema; original preserved"
        );

        library.id = manifest.id;
        library.checked_path(Path::new("songs"), true)?;
        Ok(library)
    }
    pub fn root(&self) -> &Path {
        &self.root
    }
    pub fn id(&self) -> LibraryId {
        self.id
    }
    pub fn asset_path(&self, record: &SongRecord) -> Result<PathBuf> {
        self.validate_asset(record)?;
        self.checked_path(Path::new(&record.asset.path), true)
    }
    fn validate_asset(&self, record: &SongRecord) -> Result<()> {
        let expected = format!("songs/{}/assets/{}/", record.id, record.asset.id);
        ensure!(
            record.asset.path.starts_with(&expected),
            "Recording path is not owned by this song and asset"
        );
        ensure!(
            Path::new(&record.asset.path).components().count() == 5,
            "Invalid managed recording path"
        );
        self.checked_path(Path::new(&record.asset.path), false)?;
        Ok(())
    }
    pub fn record(&self, key: &str) -> Result<ManagedSong> {
        crate::project::SongId::try_from(key.to_owned())?;
        let song: ManagedSong = self.read(&format!("songs/{key}/song.json"))?;
        self.validate_song(&song)?;
        ensure!(
            song.record.id.to_string() == key,
            "Song filename identity mismatch"
        );
        Ok(song)
    }
    fn validate_song(&self, song: &ManagedSong) -> Result<()> {
        ensure!(
            song.version == 1,
            "Unsupported managed song schema; original preserved"
        );
        song.record.validate()?;
        self.validate_asset(&song.record)?;
        ensure!(
            song.metadata.clone().normalized()? == song.metadata,
            "Invalid song metadata"
        );
        ensure!(
            song.date_added <= song.last_opened && song.last_opened <= 8_640_000_000_000_000,
            "Invalid song dates"
        );
        Ok(())
    }
    pub fn catalogue(&self) -> Result<Catalogue> {
        // The list is a projection: authoritative documents preserve all metadata.
        let mut catalogue = Catalogue::default();
        for entry in fs::read_dir(self.checked_path(Path::new("songs"), true)?)? {
            let entry = entry?;
            if !entry.file_type()?.is_dir() {
                continue;
            }
            let key = entry.file_name().to_string_lossy().into_owned();
            if crate::project::SongId::try_from(key.clone()).is_err() {
                continue;
            }
            if !entry.path().join("song.json").exists() {
                continue;
            }
            catalogue.songs.push(self.entry(&self.record(&key)?)?);
        }
        catalogue.validate()?;
        Ok(catalogue)
    }
    fn entry(&self, song: &ManagedSong) -> Result<Entry> {
        Ok(Entry {
            song_key: song.record.id.to_string(),
            metadata: song.metadata.clone(),
            file_location: self
                .checked_path(Path::new(&song.record.asset.path), false)?
                .to_string_lossy()
                .into_owned(),
            date_added: song.date_added,
            last_opened: song.last_opened,
        })
    }
    fn save_song_document(&self, song: &ManagedSong) -> Result<()> {
        self.validate_song(song)?;
        let path = format!("songs/{}/song.json", song.record.id);
        if self.root.join(&path).exists() {
            self.record(&song.record.id.to_string())?;
        }
        self.write(&path, song)
    }
    fn save_song(&self, song: &ManagedSong) -> Result<Vec<String>> {
        self.save_song_document(song)?;
        let mut warnings = vec![];
        if let Err(error) = self.save_projection() {
            warnings.push(format!(
                "Song saved, but catalogue projection could not be updated: {error:#}"
            ));
        }
        Ok(warnings)
    }
    fn save_projection(&self) -> Result<()> {
        if self.root.join("catalogue.json").exists() {
            let old: Catalogue = self.read("catalogue.json")?;
            old.validate()?;
        }
        self.write("catalogue.json", &self.catalogue()?)
    }
    pub fn save_record(&self, record: &SongRecord) -> Result<Vec<String>> {
        let mut song = self.record(&record.id.to_string())?;
        song.record = record.clone();
        self.save_song(&song)
    }
    pub fn update_metadata(&self, key: &str, metadata: Metadata) -> Result<Catalogue> {
        let mut song = self.record(key)?;
        song.metadata = metadata.normalized()?;
        let warnings = self.save_song(&song)?;
        ensure!(warnings.is_empty(), "{}", warnings.join("; "));
        self.catalogue()
    }
    pub fn duplicates(&self, digest: &str) -> Result<Vec<Entry>> {
        self.catalogue()?
            .songs
            .into_iter()
            .filter_map(|entry| match self.record(&entry.song_key) {
                Ok(song) if song.record.asset.digest == digest => Some(Ok(entry)),
                Ok(_) => None,
                Err(error) => Some(Err(error)),
            })
            .collect()
    }
    pub fn delete(&self, key: &str) -> Result<Vec<String>> {
        self.record(key)?;
        let directory = self.checked_path(Path::new(&format!("songs/{key}")), true)?;
        paths::reject_tree_symlinks(&directory)?;
        fs::remove_dir_all(directory).context("Could not completely delete song")?;
        Ok(self
            .save_projection()
            .err()
            .map(|e| format!("Song deleted, but catalogue projection update failed: {e:#}"))
            .into_iter()
            .collect())
    }
    fn read<T: serde::de::DeserializeOwned>(&self, relative: &str) -> Result<T> {
        let path = self.checked_path(Path::new(relative), true)?;
        let metadata = fs::metadata(&path)?;
        ensure!(
            metadata.is_file() && metadata.len() <= 16 * 1024 * 1024,
            "Invalid or oversized library document; original preserved"
        );
        serde_json::from_slice(&fs::read(path)?)
            .context("Invalid library document; original preserved")
    }
    fn write<T: Serialize>(&self, relative: &str, value: &T) -> Result<()> {
        let path = self.checked_path(Path::new(relative), false)?;
        let temporary = path.with_extension("json.tmp");
        let rel = temporary.strip_prefix(&self.root)?;
        self.checked_path(rel, false)?;
        let bytes = serde_json::to_vec_pretty(value)?;
        ensure!(
            bytes.len() <= 16 * 1024 * 1024,
            "Library document exceeds storage limit"
        );
        crate::storage::atomic_write(&path, &bytes)
    }
}
