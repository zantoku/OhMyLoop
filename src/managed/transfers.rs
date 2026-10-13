use super::*;
use crate::{
    audio::Audio,
    project::{AudioAssetId, asset_digest},
};
use sha2::{Digest, Sha256};
use std::{
    io::{Read, Write},
    sync::Arc,
};
#[derive(Clone)]
pub struct Prepared {
    pub record: SongRecord,
    pub audio: Arc<Audio>,
    pub metadata: Metadata,
    pub warnings: Vec<String>,
}
#[derive(Clone)]
pub struct PreparedReplacement {
    pub prepared: Prepared,
    expected_revision: u64,
    timing_unchanged: bool,
}
impl ManagedLibrary {
    fn copy_recording(
        &self,
        source: &Path,
        record: &mut SongRecord,
        progress: &mut impl FnMut(u64, u64),
    ) -> Result<Arc<Audio>> {
        let name = source
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("recording.wav");
        let name: String = name
            .chars()
            .map(|c| {
                if c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_') {
                    c
                } else {
                    '_'
                }
            })
            .take(200)
            .collect();
        let name = if name == "." || name == ".." || name.is_empty() {
            "recording.wav".to_owned()
        } else {
            name
        };
        record.asset.path = format!("songs/{}/assets/{}/{name}", record.id, record.asset.id);
        self.validate_asset(record)?;
        let destination = self.checked_path(Path::new(&record.asset.path), false)?;
        let result = (|| {
            fs::create_dir_all(destination.parent().context("Missing recording parent")?)?;
            let mut input = fs::File::open(source)?;
            let total = input.metadata()?.len();
            ensure!(
                input.metadata()?.is_file(),
                "Import source is not a regular file"
            );
            let mut output = fs::OpenOptions::new()
                .create_new(true)
                .write(true)
                .open(&destination)?;
            let mut buffer = vec![0; 256 * 1024];
            let mut copied = 0;
            let mut digest = Sha256::new();
            progress(0, total);
            loop {
                let count = input.read(&mut buffer)?;
                if count == 0 {
                    break;
                }
                output.write_all(&buffer[..count])?;
                digest.update(&buffer[..count]);
                copied += count as u64;
                progress(copied, total);
            }
            output.sync_all()?;
            drop(output);
            record.asset.digest = format!("{:x}", digest.finalize());
            Ok(Arc::new(Audio::load(&destination)?))
        })();
        match result {
            Ok(audio) => Ok(audio),
            Err(error) => Err(with_cleanup(error, self.cleanup_asset(record))),
        }
    }
    pub fn import(&self, source: &Path, mut progress: impl FnMut(u64, u64)) -> Result<Prepared> {
        let digest = "0".repeat(64);
        let metadata = crate::library::read_tags(source)?.normalized()?;
        // Only identity/path are used before decoding the copied file.
        let mut record = SongRecord::new(
            "pending".into(),
            digest,
            &Audio {
                rate: 1,
                frames: vec![[0.0, 0.0]],
            },
        );
        let audio = match self.copy_recording(source, &mut record, &mut progress) {
            Ok(audio) => audio,
            Err(error) => {
                return Err(with_cleanup(error, self.cleanup_uncommitted_song(&record)))
                    .context("Import failed");
            }
        };
        // Keep the allocated identities, initialize geometry from actual decoded audio.
        let id = record.id;
        let asset_id = record.asset.id;
        record = SongRecord::new(record.asset.path, record.asset.digest, &audio);
        record.id = id;
        record.asset.id = asset_id;
        Ok(Prepared {
            record,
            audio,
            metadata,
            warnings: vec![],
        })
    }
    pub fn commit_import(&self, mut prepared: Prepared) -> Result<Prepared> {
        let path = format!("songs/{}/song.json", prepared.record.id);
        ensure!(!self.root.join(&path).exists(), "Song already exists");
        let now = now_ms()?;
        let song = ManagedSong {
            version: 1,
            record: prepared.record.clone(),
            metadata: prepared.metadata.clone(),
            date_added: now,
            last_opened: now,
            stems: None,
        };
        match self.save_song(&song) {
            Ok(warnings) => prepared.warnings.extend(warnings),
            Err(error) => {
                return Err(with_cleanup(
                    error,
                    self.cleanup_uncommitted_song(&prepared.record),
                ))
                .context("Could not save imported song");
            }
        }
        Ok(prepared)
    }
    pub fn discard_prepared(&self, prepared: &Prepared) -> Result<()> {
        if self
            .root
            .join(format!("songs/{}/song.json", prepared.record.id))
            .exists()
        {
            let current = self.record(&prepared.record.id.to_string())?;
            ensure!(
                current.record.asset.id != prepared.record.asset.id,
                "Cannot discard the current committed recording"
            );
            self.cleanup_asset(&prepared.record)
        } else {
            self.cleanup_uncommitted_song(&prepared.record)
        }
    }
    fn cleanup_uncommitted_song(&self, record: &SongRecord) -> Result<()> {
        let directory = self.checked_path(Path::new(&format!("songs/{}", record.id)), false)?;
        if !directory.exists() {
            return Ok(());
        }
        ensure!(
            !directory.join("song.json").exists(),
            "Cannot clean a committed song"
        );
        paths::reject_tree_symlinks(&directory)?;
        fs::remove_dir_all(directory)?;
        Ok(())
    }
    fn cleanup_asset(&self, record: &SongRecord) -> Result<()> {
        self.validate_asset(record)?;
        let directory = self.checked_path(
            Path::new(&format!("songs/{}/assets/{}", record.id, record.asset.id)),
            false,
        )?;
        if directory.exists() {
            paths::reject_tree_symlinks(&directory)?;
            fs::remove_dir_all(directory)?;
        }
        Ok(())
    }
    pub fn prepare_replace(
        &self,
        key: &str,
        source: &Path,
        timing_unchanged: bool,
        mut progress: impl FnMut(u64, u64),
    ) -> Result<PreparedReplacement> {
        let song = self.record(key)?;
        let expected_revision = song.record.reference_revision;
        let mut record = song.record;
        record.asset.id = AudioAssetId::new();
        let audio = self.copy_recording(source, &mut record, &mut progress)?;
        let path = record.asset.path.clone();
        let copied_id = record.asset.id;
        if let Err(error) =
            record.replace(path, record.asset.digest.clone(), &audio, timing_unchanged)
        {
            return Err(with_cleanup(error, self.cleanup_asset(&record)));
        }
        record.asset.id = copied_id;
        Ok(PreparedReplacement {
            prepared: Prepared {
                record,
                audio,
                metadata: song.metadata,
                warnings: vec![],
            },
            expected_revision,
            timing_unchanged,
        })
    }
    pub fn commit_replace(&self, mut replacement: PreparedReplacement) -> Result<Prepared> {
        let old = self.record(&replacement.prepared.record.id.to_string())?;
        if old.record.reference_revision != replacement.expected_revision {
            self.discard_prepared(&replacement.prepared)?;
            anyhow::bail!("Recording changed while replacement was copying");
        }
        let mut song = old.clone();
        let prepared = &replacement.prepared;
        song.record.replace(
            prepared.record.asset.path.clone(),
            prepared.record.asset.digest.clone(),
            &prepared.audio,
            replacement.timing_unchanged,
        )?;
        song.record.asset.id = prepared.record.asset.id;
        song.stems = None;
        match self.save_song(&song) {
            Ok(warnings) => replacement.prepared.warnings.extend(warnings),
            Err(error) => {
                return Err(with_cleanup(
                    error,
                    self.discard_prepared(&replacement.prepared),
                ));
            }
        }
        if let Err(error) = self.cleanup_stems(&old.record.id.to_string(), old.stems.as_ref()) {
            replacement.prepared.warnings.push(format!(
                "Recording replaced, but stems/cache cleanup failed: {error:#}"
            ));
        }
        if let Err(error) = self.cleanup_asset(&old.record) {
            replacement.prepared.warnings.push(format!(
                "Recording replaced, but previous recording cleanup failed: {error:#}"
            ));
        }
        replacement.prepared.record = song.record;
        replacement.prepared.metadata = song.metadata;
        Ok(replacement.prepared)
    }
    pub fn open_song(&self, key: &str) -> Result<Prepared> {
        let song = self.record(key)?;
        let path = self.asset_path(&song.record)?;
        ensure!(
            asset_digest(&path)? == song.record.asset.digest,
            "Recording changed outside OhMyLoop; explicitly replace the reference recording before playback"
        );
        let audio = Arc::new(Audio::load(&path)?);
        ensure!(
            audio.rate == song.record.asset.rate
                && audio.frames.len() as u64 == song.record.asset.frames,
            "Recording geometry changed; replace reference recording"
        );
        Ok(Prepared {
            record: song.record,
            audio,
            metadata: song.metadata,
            warnings: vec![],
        })
    }
    pub fn mark_opened(&self, key: &str) -> Result<Vec<String>> {
        let mut song = self.record(key)?;
        song.last_opened = now_ms()?.max(song.last_opened);
        self.save_song(&song)
    }
    pub fn save_document(&self, document: &crate::project::Document) -> Result<Vec<String>> {
        let mut song = self.record(&document.song_key)?;
        song.record.update_document(document)?;
        self.save_song_document(&song)?;
        Ok(vec![])
    }
}

fn with_cleanup(error: anyhow::Error, cleanup: Result<()>) -> anyhow::Error {
    match cleanup {
        Ok(()) => error,
        Err(cleanup) => error.context(format!("Immediate copy cleanup also failed: {cleanup:#}")),
    }
}
