mod managed;
pub mod stems;
use crate::{
    audio::Audio,
    count_in::Settings as CountInSettings,
    playback::Player,
    project::{Document, Reference, SessionId, Song, SongRecord, Store, asset_digest, waveform},
};
use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};
use std::{
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::Ordering,
        mpsc::{self, SyncSender},
    },
    thread,
    time::{Duration, Instant},
};

const DEMO: &str = "builtin-demo";

#[derive(Clone, Serialize)]
pub struct Transport {
    pub song_key: Option<String>,
    pub reference_token: Option<String>,
    pub position: f64,
    pub playing: bool,
    pub finished: bool,
    pub counting_in: bool,
    pub audio_error: Option<String>,
    pub save_error: Option<String>,
    pub device: Option<String>,
    pub musical_position: Option<crate::timing::Position>,
}

#[derive(Serialize)]
pub struct Snapshot {
    pub library_status: LibraryStatus,
    pub song: Option<Song>,
    pub document: Option<Document>,
    pub transport: Transport,
    pub map_view: Option<crate::timing::MapView>,
    pub library_error: Option<String>,
    pub startup_notice: Option<String>,
}

#[derive(Clone, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Action {
    TogglePlay,
    Pause,
    Seek { seconds: f64 },
    RetryAudio,
}

#[derive(Clone, Serialize)]
pub struct LibraryStatus {
    pub selected: Option<crate::managed::LibraryPointer>,
    pub available: bool,
    pub error: Option<String>,
    pub import_notice_seen: bool,
}

enum Command {
    StemStatus(String),
    BeginStems(String),
    SeparationContext(String),
    BeginSeparation(String),
    FinishSeparation,
    BeginStemRefresh(String),
    FinishStems,
    CommitStems(stems::Operation, stems::Activation),
    RefreshStems(stems::Operation, stems::Activation),
    RemoveStems(String, String),
    MixStems(String, String, Vec<stems::TrackControl>, bool),
    ManagedStatus,
    ManagedSnapshot,
    ManagedLastSong,
    ManagedContext,
    BeginManagedOpen,
    SelectManaged(crate::managed::ManagedLibrary, bool),
    CommitManaged(u64, crate::managed::Prepared),
    ActivateManaged(u64, u64, stems::PreparedOpen),
    CommitReplacement(u64, crate::managed::PreparedReplacement, Option<String>),
    ReplacementContext(String),
    DeleteManaged(String),
    ExplainImport,
    Restore,
    Load(String),
    Separate(String),
    Replace(String, String, bool),
    LibraryReference(String),
    ReplaceLibrary(String, String, String, bool),
    Library,
    OpenLibrary(String),
    UpdateLibrary(String, crate::library::Metadata),
    Update(Document),
    Act(Action, String),
    Status,
    Shutdown,
    Source(String),
    Metronome(String, bool, f32, Option<crate::timing::MeasureMap>),
    CountIn(String, CountInSettings),
}
enum Response {
    StemStatus(stems::Status),
    StemOperation(Box<stems::Operation>),
    ManagedStatus(LibraryStatus),
    ManagedLastSong(Option<String>),
    ReplacementContext(bool),
    ManagedContext(crate::managed::ManagedLibrary, u64),
    ManagedOpenContext(crate::managed::ManagedLibrary, u64, u64),
    Snapshot(Box<Snapshot>),
    Status(Transport),
    Source(Arc<Audio>),
    Library(crate::library::Catalogue),
    Reference(String),
}
struct Request {
    command: Command,
    reply: SyncSender<std::result::Result<Response, String>>,
}

/// The controller owns CPAL's platform-specific stream on one dedicated thread.
/// It is safe to share this handle with Tauri command tasks; no stream or DSP
/// object needs an unsafe Send implementation.
#[derive(Clone)]
pub struct Desktop {
    requests: SyncSender<Request>,
}

impl Desktop {
    pub fn source(&self, reference_token: String) -> Result<Arc<Audio>> {
        match self.request(Command::Source(reference_token))? {
            Response::Source(audio) => Ok(audio),
            _ => unreachable!(),
        }
    }
    pub fn start(directory: PathBuf) -> Result<Self> {
        let library = crate::library::Store::new(&directory);
        let preferences = crate::managed::Preferences::new(&directory);
        let preference_data = preferences.read()?;
        let (managed, root_error) = if let Some(pointer) = &preference_data.active {
            match crate::managed::ManagedLibrary::open(Path::new(&pointer.root)) {
                Ok(library) if library.id() == pointer.id => (Some(library), None),
                Ok(_) => (
                    None,
                    Some("Selected folder belongs to another library".into()),
                ),
                Err(error) => (None, Some(format!("{error:#}"))),
            }
        } else {
            (None, None)
        };
        let store = Store::new(directory)?;
        let (requests, receiver) = mpsc::sync_channel::<Request>(8);
        thread::Builder::new()
            .name("ohmyloop-control".into())
            .spawn(move || {
                let mut controller = Controller {
                    preferences,
                    preference_data,
                    managed,
                    root_error,
                    managed_generation: 0,
                    activation_generation: 0,
                    store,
                    library,
                    library_error: None,
                    song: None,
                    audio: None,
                    document: None,
                    player: None,
                    audio_error: None,
                    save_error: None,
                    last_save: Instant::now(),
                    review_map: None,
                    metronome_enabled: false,
                    metronome_volume: 0.5,
                    count_in: CountInSettings::default(),
                    stems: None,
                    stems_busy: false,
                    separation_busy: false,
                };
                while let Ok(request) = receiver.recv() {
                    if matches!(request.command, Command::Shutdown) {
                        controller.persist_position();
                        let _ = request
                            .reply
                            .send(Ok(Response::Status(controller.transport())));
                        return;
                    }
                    let result = controller
                        .request(request.command)
                        .map_err(|e| format!("{e:#}"));
                    let _ = request.reply.send(result);
                }
                controller.persist_position();
            })?;
        Ok(Self { requests })
    }
    fn request(&self, command: Command) -> Result<Response> {
        let (reply, response) = mpsc::sync_channel(1);
        self.requests.send(Request { command, reply })?;
        response
            .recv_timeout(Duration::from_secs(60))
            .context("The audio controller did not respond")?
            .map_err(anyhow::Error::msg)
    }
    fn snapshot(&self, command: Command) -> Result<Snapshot> {
        match self.request(command)? {
            Response::Snapshot(s) => Ok(*s),
            _ => unreachable!(),
        }
    }
    pub fn library(&self) -> Result<crate::library::Catalogue> {
        match self.request(Command::Library)? {
            Response::Library(catalogue) => Ok(catalogue),
            _ => unreachable!(),
        }
    }
    pub fn open_library(&self, key: String) -> Result<Snapshot> {
        self.open_library_with_progress(key, |_, _| {})
    }
    pub fn open_library_with_progress(
        &self,
        key: String,
        mut progress: impl FnMut(String, crate::stems::PreparationProgress),
    ) -> Result<Snapshot> {
        if self.managed_status()?.selected.is_some() {
            let (library, generation, open_generation) = self.begin_managed_open()?;
            let prepared = library.open_song(&key)?;
            let token = prepared.record.reference(SessionId::new()).token();
            let prepared =
                Self::prepare_stem_open(&library, prepared, |p| progress(token.clone(), p));
            return self.snapshot(Command::ActivateManaged(
                generation,
                open_generation,
                prepared,
            ));
        }
        self.snapshot(Command::OpenLibrary(key))
    }
    pub fn update_library(
        &self,
        key: String,
        metadata: crate::library::Metadata,
    ) -> Result<crate::library::Catalogue> {
        match self.request(Command::UpdateLibrary(key, metadata))? {
            Response::Library(catalogue) => Ok(catalogue),
            _ => unreachable!(),
        }
    }
    pub fn restore_managed(&self) -> Result<Snapshot> {
        self.restore_managed_with_progress(|_, _| {})
    }
    pub fn restore_managed_with_progress(
        &self,
        progress: impl FnMut(String, crate::stems::PreparationProgress),
    ) -> Result<Snapshot> {
        let status = self.managed_status()?;
        if status.selected.is_none() {
            return self.snapshot(Command::ManagedSnapshot);
        }
        self.restore_with_progress(progress)
    }
    pub fn restore(&self) -> Result<Snapshot> {
        self.restore_with_progress(|_, _| {})
    }
    fn restore_with_progress(
        &self,
        mut progress: impl FnMut(String, crate::stems::PreparationProgress),
    ) -> Result<Snapshot> {
        let initial = self.snapshot(Command::Restore)?;
        if initial.library_status.available {
            let (library, generation, open_generation) = self.begin_managed_open()?;
            if let Some(key) = self.managed_last_song()? {
                let prepared = library.open_song(&key)?;
                let token = prepared.record.reference(SessionId::new()).token();
                let prepared =
                    Self::prepare_stem_open(&library, prepared, |p| progress(token.clone(), p));
                return self.snapshot(Command::ActivateManaged(
                    generation,
                    open_generation,
                    prepared,
                ));
            }
        }
        Ok(initial)
    }
    pub fn load(&self, path: Option<String>) -> Result<Snapshot> {
        self.snapshot(Command::Load(path.unwrap_or_else(|| DEMO.into())))
    }
    pub fn update(&self, document: Document) -> Result<Snapshot> {
        self.snapshot(Command::Update(document))
    }
    pub fn act(&self, action: Action, reference_token: String) -> Result<Snapshot> {
        self.act_with_progress(action, reference_token, |_| {})
    }
    pub fn act_with_progress(
        &self,
        action: Action,
        reference_token: String,
        progress: impl FnMut(crate::stems::PreparationProgress),
    ) -> Result<Snapshot> {
        if matches!(action, Action::RetryAudio)
            && self.stems_status(reference_token.clone())?.set_id.is_some()
        {
            self.refresh_stems(reference_token.clone(), progress)?;
        }
        self.snapshot(Command::Act(action, reference_token))
    }
    pub fn load_separate(&self, path: String) -> Result<Snapshot> {
        self.snapshot(Command::Separate(path))
    }
    pub fn replace_reference(
        &self,
        token: String,
        path: String,
        timing_unchanged: bool,
    ) -> Result<Snapshot> {
        if self.managed_status()?.selected.is_some() {
            self.source(token.clone())?;
            let expected = Reference::parse(&token)?;
            return self.replace_library_reference(
                expected.song.to_string(),
                token,
                path,
                timing_unchanged,
            );
        }
        self.snapshot(Command::Replace(token, path, timing_unchanged))
    }
    pub fn library_reference(&self, key: String) -> Result<String> {
        match self.request(Command::LibraryReference(key))? {
            Response::Reference(token) => Ok(token),
            _ => unreachable!(),
        }
    }
    pub fn replace_library_reference(
        &self,
        key: String,
        token: String,
        path: String,
        timing_unchanged: bool,
    ) -> Result<Snapshot> {
        if self.managed_status()?.selected.is_some() {
            let (library, generation) = self.managed_context()?;
            let expected = Reference::parse(&token)?;
            let record = library.record(&key)?.record;
            ensure!(
                expected.song == record.id
                    && expected.asset == record.asset.id
                    && expected.timeline == record.timeline.id
                    && expected.revision == record.reference_revision,
                "Recording changed; reopen replacement choices"
            );
            let active = match self.request(Command::ReplacementContext(token.clone()))? {
                Response::ReplacementContext(active) => active,
                _ => unreachable!(),
            };
            let prepared =
                library.prepare_replace(&key, Path::new(&path), timing_unchanged, |_, _| {})?;
            let cleanup = prepared.prepared.clone();
            let result = self.snapshot(Command::CommitReplacement(
                generation,
                prepared,
                active.then_some(token),
            ));
            if result.is_err() {
                let _ = library.discard_prepared(&cleanup);
            }
            return result;
        }
        self.snapshot(Command::ReplaceLibrary(key, token, path, timing_unchanged))
    }

    pub fn status(&self) -> Result<Transport> {
        match self.request(Command::Status)? {
            Response::Status(s) => Ok(s),
            _ => unreachable!(),
        }
    }
    pub fn metronome(
        &self,
        reference_token: String,
        enabled: bool,
        volume: f32,
        map: Option<crate::timing::MeasureMap>,
    ) -> Result<Transport> {
        match self.request(Command::Metronome(reference_token, enabled, volume, map))? {
            Response::Status(s) => Ok(s),
            _ => unreachable!(),
        }
    }
    pub fn count_in(
        &self,
        reference_token: String,
        enabled: bool,
        repeat: bool,
        volume: f32,
    ) -> Result<Transport> {
        match self.request(Command::CountIn(
            reference_token,
            CountInSettings {
                enabled,
                repeat,
                volume,
            },
        ))? {
            Response::Status(s) => Ok(s),
            _ => unreachable!(),
        }
    }
    pub fn shutdown(&self) {
        let _ = self.request(Command::Shutdown);
    }
}

struct Controller {
    preferences: crate::managed::Preferences,
    preference_data: crate::managed::PreferenceData,
    managed: Option<crate::managed::ManagedLibrary>,
    root_error: Option<String>,
    managed_generation: u64,
    activation_generation: u64,
    store: Store,
    library: crate::library::Store,
    library_error: Option<String>,
    song: Option<Song>,
    audio: Option<Arc<Audio>>,
    document: Option<Document>,
    player: Option<Player>,
    audio_error: Option<String>,
    save_error: Option<String>,
    last_save: Instant,
    review_map: Option<crate::timing::MeasureMap>,
    metronome_enabled: bool,
    metronome_volume: f32,
    count_in: CountInSettings,
    stems: Option<stems::Activation>,
    stems_busy: bool,
    separation_busy: bool,
}

impl Controller {
    fn request(&mut self, command: Command) -> Result<Response> {
        if self.separation_busy {
            ensure!(
                matches!(
                    command,
                    Command::Status
                        | Command::SeparationContext(_)
                        | Command::Source(_)
                        | Command::ManagedStatus
                        | Command::StemStatus(_)
                        | Command::Update(_)
                        | Command::FinishSeparation
                        | Command::CommitStems(..)
                        | Command::Library
                        | Command::ManagedSnapshot
                ),
                "Stem separation is running; recording and playback changes are disabled"
            );
        }
        if self.stems_busy {
            ensure!(
                matches!(
                    command,
                    Command::Status
                        | Command::Source(_)
                        | Command::ManagedStatus
                        | Command::StemStatus(_)
                        | Command::FinishStems
                        | Command::CommitStems(..)
                        | Command::RefreshStems(..)
                ),
                "Stem operation in progress; wait until it completes"
            );
        }
        if let Some(response) = self.stem_command(&command)? {
            return Ok(response);
        }
        let include_song = matches!(
            command,
            Command::Restore
                | Command::SelectManaged(..)
                | Command::CommitManaged(..)
                | Command::ActivateManaged(..)
                | Command::CommitReplacement(..)
                | Command::DeleteManaged(..)
                | Command::Load(_)
                | Command::OpenLibrary(_)
                | Command::Separate(_)
                | Command::Replace(..)
                | Command::ReplaceLibrary(..)
        );
        match command {
            Command::StemStatus(_)
            | Command::BeginStems(_)
            | Command::BeginStemRefresh(_)
            | Command::SeparationContext(_)
            | Command::BeginSeparation(_)
            | Command::FinishSeparation
            | Command::FinishStems
            | Command::CommitStems(..)
            | Command::RefreshStems(..)
            | Command::RemoveStems(..)
            | Command::MixStems(..) => unreachable!(),
            Command::ManagedSnapshot => {
                return Ok(Response::Snapshot(Box::new(self.snapshot(true))));
            }
            Command::ManagedLastSong => {
                return Ok(Response::ManagedLastSong(
                    self.preference_data.last_song.clone(),
                ));
            }
            Command::ManagedStatus => return Ok(Response::ManagedStatus(self.managed_status())),
            Command::BeginManagedOpen => {
                ensure!(
                    self.managed_status().available,
                    "Selected library is unavailable"
                );
                if let Some(player) = &self.player {
                    player.pause();
                }
                self.persist_position();
                ensure!(
                    self.save_error.is_none(),
                    "Practice settings could not be saved"
                );
                self.player = None;
                if let Some(stems) = &mut self.stems {
                    stems.prepared = None;
                }
                self.activation_generation = self.activation_generation.wrapping_add(1);
                return Ok(Response::ManagedOpenContext(
                    self.managed.clone().context("Select a library first")?,
                    self.managed_generation,
                    self.activation_generation,
                ));
            }
            Command::ManagedContext => {
                ensure!(
                    self.managed_status().available,
                    "Selected library is unavailable; Retry or Locate library"
                );
                self.persist_position();
                if let Some(error) = &self.save_error {
                    anyhow::bail!("{error}");
                }
                return Ok(Response::ManagedContext(
                    self.managed.clone().context("Select a library first")?,
                    self.managed_generation,
                ));
            }
            Command::ExplainImport => {
                let mut prefs = self.preference_data.clone();
                prefs.import_explained = true;
                self.preferences.save(&prefs)?;
                self.preference_data = prefs;
            }
            Command::SelectManaged(library, locate) => {
                if locate {
                    ensure!(
                        self.preference_data
                            .active
                            .as_ref()
                            .is_some_and(|p| p.id == library.id()),
                        "This is a different library; use Open existing library"
                    );
                }
                self.persist_position();
                if let Some(error) = &self.save_error {
                    anyhow::bail!("{error}");
                }
                let mut prefs = self.preference_data.clone();
                prefs.active = Some(crate::managed::LibraryPointer {
                    id: library.id().to_owned(),
                    root: library.root().to_string_lossy().into_owned(),
                });
                prefs.last_song = None;
                self.preferences.save(&prefs)?;
                self.preference_data = prefs;
                self.managed = Some(library);
                self.managed_generation = self.managed_generation.wrapping_add(1);
                self.root_error = None;
                self.unload();
            }
            Command::CommitManaged(id, prepared) => {
                let library = self.managed.clone().context("Library is unavailable")?;
                ensure!(
                    self.managed_generation == id && self.managed_status().available,
                    "Library changed; discard stale import"
                );
                self.persist_position();
                if let Some(error) = &self.save_error {
                    anyhow::bail!("{error}");
                }
                let prepared = library.commit_import(prepared)?;
                self.activate_managed(&library, prepared, None)?;
            }
            Command::ActivateManaged(id, open_generation, prepared) => {
                ensure!(
                    self.activation_generation == open_generation,
                    "Song changed; discard stale open"
                );
                let library = self.managed.clone().context("Library is unavailable")?;
                ensure!(
                    self.managed_generation == id && self.managed_status().available,
                    "Library changed; discard stale open"
                );
                self.persist_position();
                if let Some(error) = &self.save_error {
                    anyhow::bail!("{error}");
                }
                self.activate_managed(&library, prepared.recording, prepared.stems)?;
            }
            Command::ReplacementContext(token) => {
                let expected = Reference::parse(&token)?;
                let active = self
                    .song
                    .as_ref()
                    .is_some_and(|s| s.key == expected.song.to_string());
                if active {
                    self.check_reference(&token)?;
                }
                return Ok(Response::ReplacementContext(active));
            }
            Command::CommitReplacement(id, replacement, active_token) => {
                if let Some(token) = &active_token {
                    self.check_reference(token)?;
                }
                let library = self.managed.clone().context("Library is unavailable")?;
                ensure!(
                    self.managed_generation == id && self.managed_status().available,
                    "Library changed; discard stale replacement"
                );
                self.persist_position();
                if let Some(error) = &self.save_error {
                    anyhow::bail!("{error}");
                }
                let prepared = library.commit_replace(replacement)?;
                self.activate_managed(&library, prepared, None)?;
            }
            Command::DeleteManaged(key) => {
                let library = self.managed.as_ref().context("Library is unavailable")?;
                let warnings = library.delete(&key)?;
                self.library_error = (!warnings.is_empty()).then(|| warnings.join("\n"));
                if self.song.as_ref().is_some_and(|song| song.key == key) {
                    self.unload();
                }
                if self.preference_data.last_song.as_deref() == Some(&key) {
                    self.preference_data.last_song = None;
                    if let Err(error) = self.preferences.save(&self.preference_data) {
                        let warning = format!(
                            "Song deleted, but last-song pointer could not be saved: {error:#}"
                        );
                        self.library_error = Some(
                            self.library_error
                                .take()
                                .map_or(warning.clone(), |old| format!("{old}\n{warning}")),
                        );
                    }
                }
            }
            Command::Restore => {
                if self.preference_data.active.is_some() {
                    return Ok(Response::Snapshot(Box::new(self.snapshot(true))));
                }
                if let Some(source) = self.store.last_song()? {
                    let location = self.store.record(&source)?.asset.path.clone();
                    self.load(&location, Some(&source), false)?;
                }
            }
            Command::Metronome(key, enabled, volume, map) => {
                let song = self.song.as_ref().context("Open a song first")?;
                ensure!(
                    song.reference_token == key,
                    "The song changed; reopen the mapping screen"
                );
                ensure!(
                    volume.is_finite() && (0.0..=1.0).contains(&volume),
                    "Invalid click volume"
                );
                if let Some(map) = &map {
                    map.validate()?;
                    let audio = self.audio.as_ref().unwrap();
                    ensure!(
                        map.rate == audio.rate && map.frames == audio.frames.len() as u64,
                        "Metronome map belongs to a different source"
                    );
                }
                let result = if let Some(player) = &self.player {
                    let ticks = if enabled {
                        map.as_ref()
                            .map(|m| crate::metronome::ticks(m, player.rate))
                            .transpose()?
                            .unwrap_or_default()
                    } else {
                        vec![]
                    };
                    player.set_metronome(enabled, volume, ticks)
                } else {
                    ensure!(!enabled, "Audio output is unavailable; choose Retry audio");
                    Ok(())
                };
                self.review_map = map;
                // A late disable still clears session intent if the worker has
                // stopped, so Retry audio cannot revive clicks outside mapping.
                self.metronome_enabled = enabled && result.is_ok();
                self.metronome_volume = volume;
                result?;
                return Ok(Response::Status(self.transport()));
            }
            Command::CountIn(key, settings) => {
                ensure!(
                    self.song.as_ref().is_some_and(|s| s.reference_token == key),
                    "The song changed; reopen Practice"
                );
                let settings = settings.validate()?;
                let map = self.document.as_ref().and_then(|d| d.measure_map.clone());
                ensure!(
                    !settings.enabled || map.as_ref().is_some_and(|m| !m.measures.is_empty()),
                    "Build a measure map before enabling count-in"
                );
                let result = if let Some(player) = &self.player {
                    player.set_count_in(settings, map)
                } else {
                    ensure!(
                        !settings.enabled,
                        "Audio output is unavailable; choose Retry audio"
                    );
                    Ok(())
                };
                self.count_in = CountInSettings {
                    enabled: settings.enabled && result.is_ok(),
                    ..settings
                };
                result?;
                return Ok(Response::Status(self.transport()));
            }
            Command::Library => {
                return Ok(Response::Library(if let Some(library) = &self.managed {
                    library.catalogue()?
                } else {
                    self.library.read()?
                }));
            }
            Command::UpdateLibrary(key, metadata) => {
                let catalogue = if let Some(library) = &self.managed {
                    library.update_metadata(&key, metadata)?
                } else {
                    self.library.update(&key, metadata)?
                };
                if let Some(song) = &mut self.song
                    && song.key == key
                {
                    song.title = catalogue.entry(&key)?.metadata.title.clone();
                }
                self.library_error = None;
                return Ok(Response::Library(catalogue));
            }
            Command::OpenLibrary(key) => {
                let location = self.library.read()?.entry(&key)?.file_location.clone();
                self.load(&location, Some(&key), false).context(
                    "Could not open library song. Use Open song to locate an unchanged file again.",
                )?;
            }
            Command::Load(source) => self.load(&source, None, false)?,
            Command::Separate(source) => self.load(&source, None, true)?,
            Command::LibraryReference(key) => {
                if let Some(song) = &self.song
                    && song.key == key
                {
                    return Ok(Response::Reference(song.reference_token.clone()));
                }
                if let Some(library) = &self.managed {
                    library.catalogue()?.entry(&key)?;
                } else {
                    self.library.read()?.entry(&key)?;
                }
                let record = if let Some(library) = &self.managed {
                    library.record(&key)?.record
                } else {
                    self.store.record(&key)?
                };
                return Ok(Response::Reference(
                    record.reference(SessionId::new()).token(),
                ));
            }
            Command::ReplaceLibrary(key, token, path, timing_unchanged) => {
                self.library.read()?.entry(&key)?;
                self.replace_recording(&key, &token, &path, timing_unchanged)?;
            }
            Command::Replace(token, path, timing_unchanged) => {
                self.check_reference(&token)?;
                let key = self.song.as_ref().unwrap().key.clone();
                self.replace_recording(&key, &token, &path, timing_unchanged)?;
            }
            Command::Update(mut document) => {
                let song = self.song.as_ref().context("Open a song first")?;
                ensure!(
                    document.reference_token == song.reference_token,
                    "Recording session changed; discard stale practice update"
                );
                document.validate(&song.key, song.duration)?;
                if self.separation_busy {
                    let previous = self.document.as_ref().unwrap();
                    ensure!(
                        document.controls == previous.controls
                            && document.loop_enabled == previous.loop_enabled
                            && document.loop_selection == previous.loop_selection,
                        "Playback controls are frozen during stem separation"
                    );
                }
                if let Some(map) = &document.measure_map {
                    let audio = self.audio.as_ref().unwrap();
                    ensure!(
                        map.rate == audio.rate && map.frames == audio.frames.len() as u64,
                        "Musical map belongs to a different decoded source"
                    );
                }
                let duration = song.duration;
                let persistent = song.path.is_some();
                document.last_position = self.transport().position;
                if persistent {
                    if let Some(library) = &self.managed {
                        let warnings = library.save_document(&document)?;
                        if !warnings.is_empty() {
                            self.library_error = Some(warnings.join("\n"));
                        }
                    } else {
                        self.store.save(&document, duration)?;
                    }
                }
                self.save_error = None;
                if let Some(player) = self.player.as_ref() {
                    let previous = self.document.as_ref().unwrap();
                    let result = (|| -> Result<()> {
                        if previous.controls != document.controls {
                            player.set_controls(document.controls)?;
                        }
                        if previous.active_loop() != document.active_loop() {
                            player.set_loop(document.active_loop())?;
                        }
                        if previous.measure_map != document.measure_map {
                            if !document
                                .measure_map
                                .as_ref()
                                .is_some_and(|m| !m.measures.is_empty())
                            {
                                self.count_in.enabled = false;
                            }
                            player.set_count_in(self.count_in, document.measure_map.clone())?;
                        }
                        Ok(())
                    })();
                    if let Err(error) = result {
                        player.pause();
                        self.audio_error = Some(format!("{error:#}"));
                    }
                }
                self.document = Some(document);
            }
            Command::Act(action, token) => {
                self.check_reference(&token)?;
                self.action(action)?;
            }
            Command::Status => {
                let status = self.transport();
                if self.last_save.elapsed() > Duration::from_secs(4) {
                    self.persist_position();
                }
                return Ok(Response::Status(status));
            }
            Command::Source(key) => {
                ensure!(
                    self.song.as_ref().is_some_and(|s| s.reference_token == key),
                    "Song changed; discard this analysis or mapping command"
                );
                return Ok(Response::Source(
                    self.audio.as_ref().context("Open a song first")?.clone(),
                ));
            }
            Command::Shutdown => unreachable!(),
        }
        Ok(Response::Snapshot(Box::new(self.snapshot(include_song))))
    }

    fn replace_recording(
        &mut self,
        key: &str,
        token: &str,
        path: &str,
        timing_unchanged: bool,
    ) -> Result<()> {
        let expected = Reference::parse(token)?;
        ensure!(
            expected.song.to_string() == key,
            "Replacement belongs to a different song"
        );
        let path = Path::new(path).canonicalize()?;
        let digest = asset_digest(&path)?;
        let audio = Audio::load(&path)?;
        ensure!(
            asset_digest(&path)? == digest,
            "Recording changed while decoding"
        );
        // Preparation completes before mutating the selected song. Recheck the
        // stored reference after the picker/decode rather than trusting its UUID.
        self.persist_position();
        let mut record = self.store.record(key)?;
        ensure!(
            record.asset.id == expected.asset
                && record.timeline.id == expected.timeline
                && record.reference_revision == expected.revision,
            "Reference recording changed; reopen replacement choices"
        );
        record.replace(
            path.to_string_lossy().into_owned(),
            digest,
            &audio,
            timing_unchanged,
        )?;
        self.activate(record, audio, false, vec![])
    }
    fn check_reference(&self, token: &str) -> Result<()> {
        Reference::parse(token)?;
        ensure!(
            self.song
                .as_ref()
                .is_some_and(|s| s.reference_token == token),
            "Recording session changed; discard stale command"
        );
        Ok(())
    }
    fn load(&mut self, source: &str, expected: Option<&str>, separate: bool) -> Result<()> {
        // Flush the active timeline before loading its record again. A replacement
        // has already committed a new reference, so never persist the old revision.
        if let Some(song) = &self.song {
            let current = song.path.is_none()
                || self.store.record(&song.key).is_ok_and(|r| {
                    r.reference_revision == song.reference_revision && r.asset.id == song.asset_id
                });
            if current {
                self.persist_position();
            }
        }
        let demo = source == DEMO;
        let (path, digest, audio) = if demo {
            (None, "0".repeat(64), Audio::demo())
        } else {
            let path = Path::new(source)
                .canonicalize()
                .context("The song file could not be found")?;
            let digest = asset_digest(&path)?;
            let audio = Audio::load(&path).context("The song could not be decoded")?;
            ensure!(
                asset_digest(&path)? == digest,
                "Recording changed while decoding; retry"
            );
            (Some(path.to_string_lossy().into_owned()), digest, audio)
        };
        let mut warnings = vec![];
        let catalogue = if demo {
            crate::library::Catalogue::default()
        } else {
            match self.library.read() {
                Ok(c) => c,
                Err(error) => {
                    warnings.push(format!("Song opened, but the library could not be read; original preserved: {error:#}"));
                    crate::library::Catalogue::default()
                }
            }
        };
        let existing = if separate || demo {
            None
        } else if let Some(key) = expected {
            Some(key.to_owned())
        } else if self.song.as_ref().is_some_and(|song| song.path == path) {
            Some(self.song.as_ref().unwrap().key.clone())
        } else {
            // A known path identifies an existing link; hashes only suggest existing exact copies.
            let known = catalogue.songs.iter().find(|e| {
                Some(&e.file_location) == path.as_ref()
                    || path.as_ref().is_some_and(|path| {
                        Path::new(&e.file_location)
                            .canonicalize()
                            .is_ok_and(|stored| stored == Path::new(path))
                    })
            });
            if let Some(entry) = known {
                Some(entry.song_key.clone())
            } else {
                let mut found = None;
                for entry in &catalogue.songs {
                    let record = match self.store.record(&entry.song_key) {
                        Ok(record) => record,
                        Err(error) => {
                            warnings.push(format!("Another library song record could not be read; original preserved: {error:#}"));
                            continue;
                        }
                    };
                    if record.asset.digest == digest {
                        found = Some(entry.song_key.clone());
                        break;
                    }
                }
                found
            }
        };
        let mut record = if let Some(key) = &existing {
            let r = self.store.record(key)?;
            ensure!(
                r.asset.digest == digest,
                "Recording changed. Use Replace recording to retain this song and choose its timeline explicitly"
            );
            ensure!(
                r.asset.rate == audio.rate && r.asset.frames == audio.frames.len() as u64,
                "Decoded recording geometry changed"
            );
            r
        } else {
            SongRecord::new(path.clone().unwrap_or_else(|| DEMO.into()), digest, &audio)
        };
        if let Some(path) = &path {
            record.asset.path = path.clone();
        }
        self.activate(record, audio, demo, warnings)
    }
    fn activate(
        &mut self,
        mut record: SongRecord,
        audio: Audio,
        demo: bool,
        mut warnings: Vec<String>,
    ) -> Result<()> {
        ensure!(
            record.asset.rate == audio.rate && record.asset.frames == audio.frames.len() as u64,
            "Decoded asset geometry differs from its record"
        );
        let path = (!demo).then(|| record.asset.path.clone());
        let session = SessionId::new();
        let mut document = record.document(session)?;
        if demo {
            document.sections = ["Verse", "Chorus", "Bridge", "Outro"]
                .iter()
                .enumerate()
                .map(|(i, name)| crate::project::Section {
                    id: format!("demo-{i}"),
                    name: (*name).into(),
                    start: i as f64 * 3.0,
                    end: (i + 1) as f64 * 3.0,
                })
                .collect();
            record.update_document(&document)?;
        }
        let title = path
            .as_ref()
            .map(|p| {
                Path::new(p)
                    .file_stem()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .into_owned()
            })
            .unwrap_or_else(|| "Practice sketch".into());
        let mut song = Song {
            key: record.id.to_string(),
            asset_id: record.asset.id,
            timeline_id: record.timeline.id,
            reference_revision: record.reference_revision,
            session_id: session,
            reference_token: record.reference(session).token(),
            title,
            path,
            duration: audio.duration(),
            rate: audio.rate,
            peaks: waveform(&audio),
        };
        if !demo {
            // The song envelope is authoritative; catalogue/restoration projections
            // can fail without leaving the committed reference and actor out of sync.
            self.store.save_record(&record)?;
            let location = record.asset.path.clone();
            let metadata = crate::library::read_tags(Path::new(&location)).unwrap_or_else(|_| {
                crate::library::Metadata {
                    title: song.title.clone(),
                    ..Default::default()
                }
            });
            match crate::library::now_ms().and_then(|now| {
                self.library
                    .opened(song.key.clone(), location, metadata, now)
            }) {
                Ok(entry) => song.title = entry.metadata.title,
                Err(error) => warnings.push(format!(
                    "Song opened, but the library could not be saved; original preserved: {error:#}"
                )),
            }
            if let Err(error) = self.store.remember(&song.key) {
                warnings.push(format!("Song opened, but last-song restoration could not be saved; original preserved: {error:#}"));
            }
        }
        self.activation_generation = self.activation_generation.wrapping_add(1);
        self.player = None;
        self.stems = None;
        self.review_map = None;
        self.metronome_enabled = false;
        self.count_in = CountInSettings::default();
        self.audio = Some(Arc::new(audio));
        self.document = Some(document);
        self.song = Some(song);
        self.save_error = None;
        self.library_error = (!warnings.is_empty()).then(|| warnings.join("\n"));
        self.connect();
        Ok(())
    }

    fn connect(&mut self) {
        self.player = None;
        let audio = self.audio.as_ref().unwrap().clone();
        let document = self.document.as_ref().unwrap();
        let result = (|| -> Result<Player> {
            let player =
                if let Some(prepared) = self.stems.as_ref().and_then(|s| s.prepared.clone()) {
                    Player::open_bounded(prepared, true, audio.rate)?
                } else {
                    Player::open_shared(audio, true)?
                };
            if let Some(stems) = &self.stems
                && stems.prepared.is_some()
            {
                player.set_mix(stems.selection(), stems.mix)?;
            }
            player.set_controls(document.controls)?;
            player.set_loop(document.active_loop())?;
            player.seek(document.last_position)?;
            let map = self.review_map.as_ref().or(document.measure_map.as_ref());
            let ticks = if self.metronome_enabled {
                map.map(|m| crate::metronome::ticks(m, player.rate))
                    .transpose()?
                    .unwrap_or_default()
            } else {
                vec![]
            };
            player.set_metronome(self.metronome_enabled, self.metronome_volume, ticks)?;
            player.set_count_in(self.count_in, document.measure_map.clone())?;
            Ok(player)
        })();
        match result {
            Ok(player) => {
                self.player = Some(player);
                self.audio_error = None;
            }
            Err(error) => {
                self.audio_error = Some(format!(
                    "Audio output is unavailable. Reconnect an output device and choose Retry audio.\n{error:#}"
                ))
            }
        }
    }

    fn action(&mut self, action: Action) -> Result<()> {
        ensure!(self.song.is_some(), "Open a song first");
        match action {
            Action::RetryAudio => self.connect(),
            Action::Pause => {
                if let Some(player) = &self.player {
                    player.pause();
                }
            }
            Action::TogglePlay => {
                let player = self
                    .player
                    .as_ref()
                    .context("Audio output is unavailable; choose Retry audio")?;
                ensure!(
                    self.audio_error.is_none(),
                    "Audio output is unavailable; choose Retry audio"
                );
                if player.shared.playing.load(Ordering::Acquire) {
                    player.pause();
                } else {
                    if player.shared.finished.load(Ordering::Acquire) {
                        player.seek(
                            self.document
                                .as_ref()
                                .unwrap()
                                .active_loop()
                                .map_or(0.0, |r| r.start),
                        )?;
                    }
                    player.play()?;
                }
            }
            Action::Seek { seconds } => {
                ensure!(
                    seconds.is_finite()
                        && (0.0..=self.song.as_ref().unwrap().duration).contains(&seconds),
                    "Seek outside song"
                );
                let document = self.document.as_mut().unwrap();
                if document
                    .active_loop()
                    .is_some_and(|r| seconds < r.start || seconds >= r.end)
                {
                    document.loop_enabled = false;
                    if let Some(player) = &self.player {
                        player.set_loop(None)?;
                    }
                }
                if let Some(player) = &self.player {
                    player.seek(seconds)?;
                }
                document.last_position = seconds;
            }
        }
        self.persist_position();
        Ok(())
    }

    fn transport(&mut self) -> Transport {
        let musical_position = |seconds: f64| {
            self.review_map
                .as_ref()
                .or_else(|| self.document.as_ref().and_then(|d| d.measure_map.as_ref()))
                .and_then(|map| map.position((seconds * f64::from(map.rate)).round() as u64))
        };
        if let Some(player) = &self.player {
            if player.shared.device_errors.load(Ordering::Relaxed) > 0
                || player.shared.worker_error.lock().unwrap().is_some()
            {
                player.pause();
                let worker = player.shared.worker_error.lock().unwrap().clone();
                self.audio_error = Some(worker.map_or_else(
                    || {
                        "Audio output stopped. Reconnect an output device and choose Retry audio."
                            .into()
                    },
                    |error| format!("Playback stopped: {error}"),
                ));
            }
            Transport {
                song_key: self.song.as_ref().map(|s| s.key.clone()),
                reference_token: self.song.as_ref().map(|s| s.reference_token.clone()),
                position: (player.shared.position() / player.rate as f64)
                    .clamp(0.0, player.duration),
                playing: player.shared.playing.load(Ordering::Acquire),
                finished: player.shared.finished.load(Ordering::Acquire),
                counting_in: player.shared.count_in.load(Ordering::Relaxed)
                    && player.shared.playing.load(Ordering::Acquire),
                audio_error: self.audio_error.clone(),
                save_error: self.save_error.clone(),
                device: Some(player.device_name.clone()),
                musical_position: musical_position(player.shared.position() / player.rate as f64),
            }
        } else {
            Transport {
                song_key: self.song.as_ref().map(|s| s.key.clone()),
                reference_token: self.song.as_ref().map(|s| s.reference_token.clone()),
                position: self.document.as_ref().map_or(0.0, |d| d.last_position),
                playing: false,
                finished: false,
                counting_in: false,
                audio_error: self.audio_error.clone(),
                save_error: self.save_error.clone(),
                device: None,
                musical_position: musical_position(
                    self.document.as_ref().map_or(0.0, |d| d.last_position),
                ),
            }
        }
    }
    fn persist_position(&mut self) {
        let position = self.transport().position;
        if let (Some(document), Some(song)) = (&mut self.document, &self.song) {
            document.last_position = position;
            if song.path.is_none() {
                self.last_save = Instant::now();
                return;
            }
            let saved = if let Some(library) = &self.managed {
                library.save_document(document).map(|_| ())
            } else {
                self.store.save(document, song.duration)
            };
            if let Err(error) = saved {
                self.save_error = Some(format!(
                    "Your latest practice settings could not be saved: {error:#}"
                ));
            } else {
                self.save_error = None;
            }
        }
        self.last_save = Instant::now();
    }
    fn snapshot(&mut self, include_song: bool) -> Snapshot {
        Snapshot {
            library_status: self.managed_status(),
            song: self.song.clone().filter(|_| include_song),
            document: self.document.clone(),
            transport: self.transport(),
            library_error: self.library_error.clone(),
            startup_notice: self.store.reset_notice().then(||"Legacy practice metadata was reset for UUID song identities. Audio files and preferences were retained.".into()),
            map_view: self
                .document
                .as_ref()
                .and_then(|d| d.measure_map.as_ref())
                .map(|m| m.view()),
        }
    }
}
