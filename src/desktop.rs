use crate::{
    audio::Audio,
    count_in::Settings as CountInSettings,
    playback::Player,
    project::{
        AudioAssetId, Document, Reference, SessionId, Song, SongRecord, Store, asset_digest,
        waveform,
    },
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

enum Command {
    Restore,
    Load(String),
    Separate(String),
    Replace(String, String, bool),
    LibraryReference(String),
    ReplaceLibrary(String, String, String, bool),
    SelectRecording(String, String),
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
        let store = Store::new(directory)?;
        let (requests, receiver) = mpsc::sync_channel::<Request>(8);
        thread::Builder::new()
            .name("ohmyloop-control".into())
            .spawn(move || {
                let mut controller = Controller {
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
    pub fn restore(&self) -> Result<Snapshot> {
        self.snapshot(Command::Restore)
    }
    pub fn load(&self, path: Option<String>) -> Result<Snapshot> {
        self.snapshot(Command::Load(path.unwrap_or_else(|| DEMO.into())))
    }
    pub fn update(&self, document: Document) -> Result<Snapshot> {
        self.snapshot(Command::Update(document))
    }
    pub fn act(&self, action: Action, reference_token: String) -> Result<Snapshot> {
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
        self.snapshot(Command::ReplaceLibrary(key, token, path, timing_unchanged))
    }
    pub fn select_recording(&self, token: String, asset_id: String) -> Result<Snapshot> {
        self.snapshot(Command::SelectRecording(token, asset_id))
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
}

impl Controller {
    fn request(&mut self, command: Command) -> Result<Response> {
        let include_song = matches!(
            command,
            Command::Restore
                | Command::Load(_)
                | Command::OpenLibrary(_)
                | Command::Separate(_)
                | Command::Replace(..)
                | Command::SelectRecording(..)
                | Command::ReplaceLibrary(..)
        );
        match command {
            Command::Restore => {
                if let Some(source) = self.store.last_song()? {
                    let location = self.store.record(&source)?.asset()?.path.clone();
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
            Command::Library => return Ok(Response::Library(self.library.read()?)),
            Command::UpdateLibrary(key, metadata) => {
                let catalogue = self.library.update(&key, metadata)?;
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
                self.library.read()?.entry(&key)?;
                let record = self.store.record(&key)?;
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
            Command::SelectRecording(token, asset) => {
                self.check_reference(&token)?;
                self.persist_position();
                let key = self.song.as_ref().unwrap().key.clone();
                let mut record = self.store.record(&key)?;
                let asset: AudioAssetId = asset.try_into()?;
                record.select_recording(asset)?;
                let path = record.asset()?.path.clone();
                ensure!(
                    asset_digest(Path::new(&path))? == record.asset()?.digest,
                    "Retained recording changed; replace it explicitly"
                );
                let audio = Audio::load(Path::new(&path))?;
                ensure!(
                    audio.rate == record.asset()?.rate
                        && audio.frames.len() as u64 == record.asset()?.frames
                        && asset_digest(Path::new(&path))? == record.asset()?.digest,
                    "Retained recording changed while decoding"
                );
                self.activate(record, audio, false, vec![])?;
            }
            Command::Update(mut document) => {
                let song = self.song.as_ref().context("Open a song first")?;
                ensure!(
                    document.reference_token == song.reference_token,
                    "Recording session changed; discard stale practice update"
                );
                document.validate(&song.key, song.duration)?;
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
                    self.store.save(&document, duration)?;
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
            record.reference.asset == expected.asset
                && record.reference.timeline == expected.timeline
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
                    r.reference_revision == song.reference_revision
                        && r.reference.asset == song.asset_id
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
                    if record.asset()?.digest == digest {
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
                r.asset()?.digest == digest,
                "Recording changed. Use Replace recording to retain this song and choose its timeline explicitly"
            );
            ensure!(
                r.asset()?.rate == audio.rate && r.asset()?.frames == audio.frames.len() as u64,
                "Decoded recording geometry changed"
            );
            r
        } else {
            SongRecord::new(path.clone().unwrap_or_else(|| DEMO.into()), digest, &audio)
        };
        if let Some(path) = &path {
            record
                .assets
                .iter_mut()
                .find(|a| a.id == record.reference.asset)
                .unwrap()
                .path = path.clone();
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
            record.asset()?.rate == audio.rate
                && record.asset()?.frames == audio.frames.len() as u64,
            "Decoded asset geometry differs from its record"
        );
        let path = (!demo).then(|| record.asset().unwrap().path.clone());
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
            asset_id: record.reference.asset,
            timeline_id: record.reference.timeline,
            reference_revision: record.reference_revision,
            session_id: session,
            reference_token: record.reference(session).token(),
            retained_recordings: record.recording_options(),
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
            let location = record.asset()?.path.clone();
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
        self.player = None;
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
            let player = Player::open_shared(audio, true)?;
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
                self.audio_error = Some(
                    "Audio output stopped. Reconnect an output device and choose Retry audio."
                        .into(),
                );
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
            if let Err(error) = self.store.save(document, song.duration) {
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
