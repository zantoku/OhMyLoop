//! Managed-library orchestration: heavy preparation runs on the command caller,
//! followed by a short actor commit with library generation and reference guards.
use super::*;

impl Desktop {
    pub(super) fn managed_last_song(&self) -> Result<Option<String>> {
        match self.request(Command::ManagedLastSong)? {
            Response::ManagedLastSong(value) => Ok(value),
            _ => unreachable!(),
        }
    }
    pub fn managed_status(&self) -> Result<LibraryStatus> {
        match self.request(Command::ManagedStatus)? {
            Response::ManagedStatus(value) => Ok(value),
            _ => unreachable!(),
        }
    }
    pub(super) fn managed_context(&self) -> Result<(crate::managed::ManagedLibrary, u64)> {
        match self.request(Command::ManagedContext)? {
            Response::ManagedContext(value, generation) => Ok((value, generation)),
            _ => unreachable!(),
        }
    }
    pub fn acknowledge_import_notice(&self) -> Result<()> {
        self.snapshot(Command::ExplainImport)?;
        Ok(())
    }
    pub fn select_library(&self, root: String, create: bool, locate: bool) -> Result<Snapshot> {
        let library = if create {
            crate::managed::ManagedLibrary::create(Path::new(&root))?
        } else {
            crate::managed::ManagedLibrary::open(Path::new(&root))?
        };
        self.snapshot(Command::SelectManaged(library, locate))
    }
    pub fn retry_library(&self) -> Result<Snapshot> {
        let pointer = self
            .managed_status()?
            .selected
            .context("Select a library first")?;
        self.select_library(pointer.root, false, true)
    }
    pub fn import_candidates(&self, source: String) -> Result<Vec<crate::library::Entry>> {
        let (library, _) = self.managed_context()?;
        library.duplicates(&asset_digest(Path::new(&source))?)
    }
    pub fn import_managed(
        &self,
        source: String,
        progress: impl FnMut(u64, u64),
    ) -> Result<Snapshot> {
        let (library, generation) = self.managed_context()?;
        let prepared = library.import(Path::new(&source), progress)?;
        let cleanup = prepared.clone();
        let result = self.snapshot(Command::CommitManaged(generation, prepared));
        if result.is_err() {
            let _ = library.discard_prepared(&cleanup);
        }
        result
    }
    pub fn delete_library_song(&self, key: String) -> Result<Snapshot> {
        self.snapshot(Command::DeleteManaged(key))
    }
}

impl Controller {
    pub(super) fn managed_status(&self) -> LibraryStatus {
        let error = self
            .preference_data
            .active
            .as_ref()
            .and_then(|pointer| {
                match crate::managed::ManagedLibrary::open(Path::new(&pointer.root)) {
                    Ok(library) if library.id() == pointer.id => None,
                    Ok(_) => Some("Selected folder belongs to another library".into()),
                    Err(error) => Some(format!("{error:#}")),
                }
            })
            .or_else(|| self.root_error.clone());
        LibraryStatus {
            selected: self.preference_data.active.clone(),
            available: self.managed.is_some() && error.is_none(),
            error,
            import_notice_seen: self.preference_data.import_explained,
        }
    }
    pub(super) fn unload(&mut self) {
        self.player = None;
        self.song = None;
        self.audio = None;
        self.document = None;
        self.review_map = None;
        self.metronome_enabled = false;
        self.count_in = CountInSettings::default();
        self.audio_error = None;
        self.save_error = None;
    }
    pub(super) fn activate_managed(
        &mut self,
        library: &crate::managed::ManagedLibrary,
        prepared: crate::managed::Prepared,
    ) -> Result<()> {
        let mut record = prepared.record;
        let latest_song = library.record(&record.id.to_string())?;
        let latest = latest_song.record;
        ensure!(
            latest.asset.id == record.asset.id
                && latest.reference_revision == record.reference_revision,
            "Recording changed; discard stale open"
        );
        record = latest;
        let audio = prepared.audio;
        let mut warnings = prepared.warnings;
        let session = SessionId::new();
        let document = record.document(session)?;
        let path = library.asset_path(&record)?.to_string_lossy().into_owned();
        let song = Song {
            key: record.id.to_string(),
            asset_id: record.asset.id,
            timeline_id: record.timeline.id,
            reference_revision: record.reference_revision,
            session_id: session,
            reference_token: record.reference(session).token(),
            title: latest_song.metadata.title,
            path: Some(path),
            duration: audio.duration(),
            rate: audio.rate,
            peaks: waveform(&audio),
        };
        match library.mark_opened(&song.key) {
            Ok(opened_warnings) => warnings.extend(opened_warnings),
            Err(error) => warnings.push(format!(
                "Song opened, but last-opened date could not be saved: {error:#}"
            )),
        }
        let mut prefs = self.preference_data.clone();
        prefs.last_song = Some(song.key.clone());
        if let Err(error) = self.preferences.save(&prefs) {
            warnings.push(format!(
                "Song opened, but last-song pointer could not be saved: {error:#}"
            ));
        } else {
            self.preference_data = prefs;
        }
        self.unload();
        self.audio = Some(audio);
        self.document = Some(document);
        self.song = Some(song);
        self.library_error = (!warnings.is_empty()).then(|| warnings.join("\n"));
        self.connect();
        Ok(())
    }
}
