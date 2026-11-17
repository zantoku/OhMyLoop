//! Guarded owned stem paths and atomic association publication.
use super::*;
use crate::stems::library::StoredStemSet;

impl ManagedLibrary {
    pub fn stem_directory(&self, key: &str, id: &str) -> Result<PathBuf> {
        crate::project::SongId::try_from(key.to_owned())?;
        uuid::Uuid::parse_str(id).context("Invalid stem-set identity")?;
        self.checked_path(Path::new(&format!("songs/{key}/stems/{id}")), false)
    }
    pub fn stem_cache_directory(&self, key: &str) -> Result<PathBuf> {
        crate::project::SongId::try_from(key.to_owned())?;
        self.checked_path(Path::new(&format!("songs/{key}/stem-cache")), false)
    }
    pub fn save_stems(
        &self,
        key: &str,
        expected_reference: u64,
        stems: Option<StoredStemSet>,
    ) -> Result<()> {
        let mut song = self.record(key)?;
        ensure!(
            song.record.reference_revision == expected_reference,
            "Recording changed; discard stale stems"
        );
        song.stems = stems;
        self.save_song_document(&song)
    }
    pub fn cleanup_stem_directory(&self, key: &str, id: &str) -> Result<()> {
        let directory = self.stem_directory(key, id)?;
        if directory.exists() {
            paths::reject_tree_symlinks(&directory)?;
            fs::remove_dir_all(directory)?;
        }
        Ok(())
    }
    pub fn cleanup_stems(&self, key: &str, old: Option<&StoredStemSet>) -> Result<()> {
        if let Some(old) = old {
            self.cleanup_stem_directory(key, &old.id)?;
        }
        let cache = self.stem_cache_directory(key)?;
        if cache.exists() {
            paths::reject_tree_symlinks(&cache)?;
            fs::remove_dir_all(cache)?;
        }
        Ok(())
    }
}
