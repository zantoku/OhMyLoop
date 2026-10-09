use crate::{
    audio::Audio,
    playback::Player,
    project::{Document, Song, Store, song_key, waveform},
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
    pub position: f64,
    pub playing: bool,
    pub finished: bool,
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
    Update(Document),
    Act(Action),
    Status,
    Shutdown,
    Source(String),
}
enum Response {
    Snapshot(Box<Snapshot>),
    Status(Transport),
    Source(Arc<Audio>),
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
    pub fn source(&self, song_key: String) -> Result<Arc<Audio>> {
        match self.request(Command::Source(song_key))? {
            Response::Source(audio) => Ok(audio),
            _ => unreachable!(),
        }
    }
    pub fn start(directory: PathBuf) -> Result<Self> {
        let store = Store::new(directory)?;
        let (requests, receiver) = mpsc::sync_channel::<Request>(8);
        thread::Builder::new()
            .name("ohmyloop-control".into())
            .spawn(move || {
                let mut controller = Controller {
                    store,
                    song: None,
                    audio: None,
                    document: None,
                    player: None,
                    audio_error: None,
                    save_error: None,
                    last_save: Instant::now(),
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
    pub fn restore(&self) -> Result<Snapshot> {
        self.snapshot(Command::Restore)
    }
    pub fn load(&self, path: Option<String>) -> Result<Snapshot> {
        self.snapshot(Command::Load(path.unwrap_or_else(|| DEMO.into())))
    }
    pub fn update(&self, document: Document) -> Result<Snapshot> {
        self.snapshot(Command::Update(document))
    }
    pub fn act(&self, action: Action) -> Result<Snapshot> {
        self.snapshot(Command::Act(action))
    }
    pub fn status(&self) -> Result<Transport> {
        match self.request(Command::Status)? {
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
    song: Option<Song>,
    audio: Option<Arc<Audio>>,
    document: Option<Document>,
    player: Option<Player>,
    audio_error: Option<String>,
    save_error: Option<String>,
    last_save: Instant,
}

impl Controller {
    fn request(&mut self, command: Command) -> Result<Response> {
        let include_song = matches!(command, Command::Restore | Command::Load(_));
        match command {
            Command::Restore => {
                if let Some(source) = self.store.last_song()? {
                    self.load(&source)?;
                }
            }
            Command::Load(source) => self.load(&source)?,
            Command::Update(mut document) => {
                let song = self.song.as_ref().context("Open a song first")?;
                document.validate(&song.key, song.duration)?;
                if let Some(map) = &document.measure_map {
                    let audio = self.audio.as_ref().unwrap();
                    ensure!(
                        map.rate == audio.rate && map.frames == audio.frames.len() as u64,
                        "Musical map belongs to a different decoded source"
                    );
                }
                document.last_position = self.transport().position;
                self.store
                    .save(&document, self.song.as_ref().unwrap().duration)?;
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
                        Ok(())
                    })();
                    if let Err(error) = result {
                        player.pause();
                        self.audio_error = Some(format!("{error:#}"));
                    }
                }
                self.document = Some(document);
            }
            Command::Act(action) => self.action(action)?,
            Command::Status => {
                let status = self.transport();
                if self.last_save.elapsed() > Duration::from_secs(4) {
                    self.persist_position();
                }
                return Ok(Response::Status(status));
            }
            Command::Source(key) => {
                ensure!(
                    self.song.as_ref().is_some_and(|s| s.key == key),
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

    fn load(&mut self, source: &str) -> Result<()> {
        let (key, title, path, audio) = if source == DEMO {
            (DEMO.into(), "Practice sketch".into(), None, Audio::demo())
        } else {
            let path = Path::new(source)
                .canonicalize()
                .context("The song file could not be found")?;
            let title = path
                .file_stem()
                .unwrap_or_default()
                .to_string_lossy()
                .into_owned();
            (
                song_key(&path)?,
                title,
                Some(path.to_string_lossy().into_owned()),
                Audio::load(&path).context("The song could not be decoded")?,
            )
        };
        let first_use = !self.store.contains(&key)?;
        let mut document = self.store.load(&key, audio.duration())?;
        // Demo has illustrative ranges on first use, not on real imported songs.
        if key == DEMO && first_use {
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
        }
        let song = Song {
            key,
            title,
            path,
            duration: audio.duration(),
            rate: audio.rate,
            peaks: waveform(&audio),
        };
        self.store.save(&document, song.duration)?;
        self.store.remember(source)?;
        self.persist_position();
        self.player = None;
        self.audio = Some(Arc::new(audio));
        self.document = Some(document);
        self.song = Some(song);
        self.save_error = None;
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
            self.document
                .as_ref()
                .and_then(|d| d.measure_map.as_ref())
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
                position: (player.shared.position() / player.rate as f64)
                    .clamp(0.0, player.duration),
                playing: player.shared.playing.load(Ordering::Acquire),
                finished: player.shared.finished.load(Ordering::Acquire),
                audio_error: self.audio_error.clone(),
                save_error: self.save_error.clone(),
                device: Some(player.device_name.clone()),
                musical_position: musical_position(player.shared.position() / player.rate as f64),
            }
        } else {
            Transport {
                song_key: self.song.as_ref().map(|s| s.key.clone()),
                position: self.document.as_ref().map_or(0.0, |d| d.last_position),
                playing: false,
                finished: false,
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
            map_view: self
                .document
                .as_ref()
                .and_then(|d| d.measure_map.as_ref())
                .map(|m| m.view()),
        }
    }
}
