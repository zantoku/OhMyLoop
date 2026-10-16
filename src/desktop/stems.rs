//! Stem sessions: heavy attachment/preparation stays outside the audio controller.
use super::*;
use crate::stems::library::{StoredStemSet, WorkflowTracker, throttled_progress};
use crate::stems::{MixState, PreparationProgress, PreparedComparison, Role};

#[derive(Clone)]
pub(super) struct PreparedOpen {
    pub recording: crate::managed::Prepared,
    pub stems: Option<Activation>,
}

#[derive(Clone)]
pub struct Activation {
    pub record: StoredStemSet,
    pub prepared: Option<Arc<PreparedComparison>>,
    pub mix: MixState,
    pub warning: Option<String>,
}
impl Activation {
    pub(super) fn selection(&self) -> usize {
        usize::from(self.record.roles().iter().any(|r| {
            let c = self.mix.tracks[r.index()];
            c.gain != 1.0 || c.mute || c.solo
        }))
    }
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TrackControl {
    pub role: String,
    pub gain: f32,
    pub mute: bool,
    pub solo: bool,
}
#[derive(Serialize)]
pub struct Status {
    pub reference_token: String,
    pub set_id: Option<String>,
    pub tracks: Vec<TrackControl>,
    pub available: bool,
    pub warning: Option<String>,
    pub attenuation: Option<f32>,
    pub model: Option<String>,
}
#[derive(Clone)]
pub struct Operation {
    pub(crate) library: crate::managed::ManagedLibrary,
    generation: u64,
    token: String,
    reference_freshness: crate::stems::source::Freshness,
    pub(crate) record: crate::managed::ManagedSong,
}
impl Operation {
    pub fn reference(&self) -> Result<PathBuf> {
        self.library.asset_path(&self.record.record)
    }
    pub fn rate(&self) -> u32 {
        self.record.record.asset.rate
    }
    pub fn frames(&self) -> u64 {
        self.record.record.asset.frames
    }
    pub fn digest(&self) -> &str {
        &self.record.record.asset.digest
    }
    pub fn owned(&self, id: &str) -> Result<PathBuf> {
        self.library
            .stem_directory(&self.record.record.id.to_string(), id)
    }
    pub fn cleanup(&self, id: &str) -> Result<()> {
        self.library
            .cleanup_stem_directory(&self.record.record.id.to_string(), id)
    }
}
impl Desktop {
    pub fn separation_context(&self, token: String) -> Result<Operation> {
        match self.request(Command::SeparationContext(token))? {
            Response::StemOperation(op) => Ok(*op),
            _ => unreachable!(),
        }
    }
    pub fn begin_separation(&self, token: String) -> Result<Operation> {
        match self.request(Command::BeginSeparation(token))? {
            Response::StemOperation(op) => Ok(*op),
            _ => unreachable!(),
        }
    }
    pub fn finish_separation(&self) -> Result<()> {
        self.request(Command::FinishSeparation)?;
        Ok(())
    }
    pub fn publish_separation(
        &self,
        operation: Operation,
        mut record: StoredStemSet,
        prepared: PreparedComparison,
    ) -> Result<Status> {
        record.inherit_saved_mix(operation.record.stems.as_ref());
        let mix = record.mix.to_mix();
        match self.request(Command::CommitStems(
            operation,
            Activation {
                record,
                prepared: Some(Arc::new(prepared)),
                mix,
                warning: None,
            },
        ))? {
            Response::StemStatus(s) => Ok(s),
            _ => unreachable!(),
        }
    }
    pub fn stems_status(&self, token: String) -> Result<Status> {
        match self.request(Command::StemStatus(token))? {
            Response::StemStatus(s) => Ok(s),
            _ => unreachable!(),
        }
    }
    pub fn set_stem_mix(
        &self,
        token: String,
        id: String,
        tracks: Vec<TrackControl>,
        persist: bool,
    ) -> Result<Status> {
        match self.request(Command::MixStems(token, id, tracks, persist))? {
            Response::StemStatus(s) => Ok(s),
            _ => unreachable!(),
        }
    }
    pub fn remove_stems(&self, token: String, id: String) -> Result<Status> {
        match self.request(Command::RemoveStems(token, id))? {
            Response::StemStatus(s) => Ok(s),
            _ => unreachable!(),
        }
    }
    pub fn attach_stems(
        &self,
        token: String,
        directory: String,
        mut progress: impl FnMut(PreparationProgress),
    ) -> Result<Status> {
        let operation = match self.request(Command::BeginStems(token))? {
            Response::StemOperation(s) => *s,
            _ => unreachable!(),
        };
        let key = operation.record.record.id.to_string();
        // New immutable directory, never mutate the currently associated set.
        let id = uuid::Uuid::new_v4().to_string();
        let owned = match operation.library.stem_directory(&key, &id) {
            Ok(path) => path,
            Err(e) => {
                let _ = self.request(Command::FinishStems);
                return Err(e);
            }
        };
        let result = (|| {
            let reference = operation.library.asset_path(&operation.record.record)?;
            let rate = Player::output_rate().unwrap_or(operation.record.record.asset.rate);
            let mut workflow = WorkflowTracker::attachment(Path::new(&directory))?;
            progress(workflow.begin_first());
            let (record, prepared) = {
                let mut publish = throttled_progress(&mut progress);
                let mut report = |event: PreparationProgress| publish(workflow.update(event));
                let mut record = crate::stems::library::attach(
                    &reference,
                    Path::new(&directory),
                    &owned,
                    rate,
                    &mut report,
                )?;
                record.id = id.clone();
                record.inherit_saved_mix(operation.record.stems.as_ref());
                // Keep the prior set alive until publication succeeds.
                let prepared = crate::stems::library::prepare(
                    &record,
                    &reference,
                    &owned,
                    &owned.join("playback-cache"),
                    rate,
                    &mut report,
                )?;
                (record, prepared)
            };
            let mix = record.mix.to_mix();
            let activation = Activation {
                record,
                prepared: Some(Arc::new(prepared)),
                mix,
                warning: None,
            };
            progress(workflow.begin("attachment.publish"));
            match self.request(Command::CommitStems(operation.clone(), activation))? {
                Response::StemStatus(s) => {
                    progress(workflow.finish());
                    Ok(s)
                }
                _ => unreachable!(),
            }
        })();
        if result.is_err() {
            let cleanup = operation.library.cleanup_stem_directory(&key, &id);
            let _ = self.request(Command::FinishStems);
            if let Err(cleanup) = cleanup {
                return result.context(format!("Uncommitted stem cleanup failed: {cleanup:#}"));
            }
        }
        result
    }
    pub(super) fn refresh_stems(
        &self,
        token: String,
        mut progress: impl FnMut(PreparationProgress),
    ) -> Result<()> {
        let operation = match self.request(Command::BeginStemRefresh(token))? {
            Response::StemOperation(s) => *s,
            _ => unreachable!(),
        };
        let result = (|| {
            let record = operation
                .record
                .stems
                .clone()
                .context("No stems attached")?;
            let key = operation.record.record.id.to_string();
            let reference = operation.library.asset_path(&operation.record.record)?;
            let owned = operation.library.stem_directory(&key, &record.id)?;
            let prepared = crate::stems::library::prepare(
                &record,
                &reference,
                &owned,
                &owned.join("playback-cache"),
                Player::output_rate().unwrap_or(record.reference_rate),
                &mut progress,
            );
            let (prepared, warning) = match prepared {
                Ok(p) => (Some(Arc::new(p)), None),
                Err(e) => (
                    None,
                    Some(format!("Stems unavailable; playing Original: {e:#}")),
                ),
            };
            let mix = record.mix.to_mix();
            self.request(Command::RefreshStems(
                operation,
                Activation {
                    record,
                    prepared,
                    mix,
                    warning,
                },
            ))?;
            Ok(())
        })();
        if result.is_err() {
            let _ = self.request(Command::FinishStems);
        }
        result
    }
    pub(super) fn prepare_stem_open(
        library: &crate::managed::ManagedLibrary,
        mut prepared: crate::managed::Prepared,
        mut progress: impl FnMut(PreparationProgress),
    ) -> PreparedOpen {
        let result = (|| -> Result<Option<Activation>> {
            let song = library.record(&prepared.record.id.to_string())?;
            let Some(record) = song.stems else {
                return Ok(None);
            };
            let mix = record.mix.to_mix();
            let prepare = (|| {
                let owned = library.stem_directory(&prepared.record.id.to_string(), &record.id)?;
                let cache = owned.join("playback-cache");
                crate::stems::library::prepare(
                    &record,
                    &library.asset_path(&prepared.record)?,
                    &owned,
                    &cache,
                    Player::output_rate().unwrap_or(record.reference_rate),
                    &mut progress,
                )
            })();
            let (audio, warning) = match prepare {
                Ok(value) => (Some(Arc::new(value)), None),
                Err(error) => (
                    None,
                    Some(format!("Stems unavailable; playing Original: {error:#}")),
                ),
            };
            Ok(Some(Activation {
                record,
                prepared: audio,
                mix,
                warning,
            }))
        })();
        let stems = match result {
            Ok(stems) => stems,
            Err(e) => {
                prepared
                    .warnings
                    .push(format!("Could not open stems: {e:#}"));
                None
            }
        };
        PreparedOpen {
            recording: prepared,
            stems,
        }
    }
}
impl Controller {
    fn stem_status(&self) -> Status {
        let token = self
            .song
            .as_ref()
            .map(|s| s.reference_token.clone())
            .unwrap_or_default();
        if let Some(stems) = &self.stems {
            Status {
                reference_token: token,
                set_id: Some(stems.record.id.clone()),
                tracks: stems
                    .record
                    .roles()
                    .iter()
                    .map(|r| {
                        let c = stems.mix.tracks[r.index()];
                        TrackControl {
                            role: r.name().into(),
                            gain: c.gain,
                            mute: c.mute,
                            solo: c.solo,
                        }
                    })
                    .collect(),
                available: stems.prepared.is_some(),
                warning: stems.warning.clone(),
                attenuation: stems.prepared.as_ref().map(|p| p.shared_attenuation),
                model: stems
                    .record
                    .provenance
                    .pointer("/model_lock/model")
                    .or_else(|| stems.record.provenance.get("backend"))
                    .and_then(|v| v.as_str())
                    .map(str::to_owned),
            }
        } else {
            Status {
                reference_token: token,
                set_id: None,
                tracks: vec![],
                available: false,
                warning: None,
                attenuation: None,
                model: None,
            }
        }
    }
    pub(super) fn stem_command(&mut self, command: &Command) -> Result<Option<Response>> {
        match command {
            Command::StemStatus(token) => {
                self.check_reference(token)?;
            }
            Command::SeparationContext(token) => {
                self.check_reference(token)?;
                let library = self
                    .managed
                    .clone()
                    .context("Separate a managed library song")?;
                let record = library.record(&self.song.as_ref().unwrap().key)?;
                let reference_freshness =
                    crate::stems::source::freshness(&library.asset_path(&record.record)?)?;
                return Ok(Some(Response::StemOperation(Box::new(Operation {
                    reference_freshness,
                    library,
                    generation: self.managed_generation,
                    token: token.clone(),
                    record,
                }))));
            }
            Command::BeginStems(token)
            | Command::BeginStemRefresh(token)
            | Command::BeginSeparation(token) => {
                self.check_reference(token)?;
                let library = self
                    .managed
                    .clone()
                    .context("Attach stems to a managed library song")?;
                if let Some(player) = &self.player {
                    player.pause();
                }
                self.persist_position();
                ensure!(
                    self.save_error.is_none(),
                    "Practice settings could not be saved"
                );
                let record = library.record(&self.song.as_ref().unwrap().key)?;
                let reference_freshness =
                    crate::stems::source::freshness(&library.asset_path(&record.record)?)?;
                if matches!(command, Command::BeginStemRefresh(_)) {
                    self.player = None;
                    if let Some(stems) = &mut self.stems {
                        stems.prepared = None;
                    }
                }
                self.activation_generation = self.activation_generation.wrapping_add(1);
                if matches!(command, Command::BeginSeparation(_)) {
                    self.separation_busy = true;
                } else {
                    self.stems_busy = true;
                }
                return Ok(Some(Response::StemOperation(Box::new(Operation {
                    reference_freshness,
                    library,
                    generation: self.managed_generation,
                    token: token.clone(),
                    record,
                }))));
            }
            Command::FinishStems => self.stems_busy = false,
            Command::FinishSeparation => self.separation_busy = false,
            Command::RefreshStems(operation, activation) => {
                self.check_reference(&operation.token)?;
                ensure!(
                    self.managed_generation == operation.generation
                        && self.stems.as_ref().map(|s| &s.record.id) == Some(&activation.record.id),
                    "Library or stems changed; discard stale preparation"
                );
                let mut activation = activation.clone();
                if let Some(old) = &self.stems {
                    activation.mix = old.mix;
                }
                self.stems = Some(activation);
                self.stems_busy = false;
            }
            Command::CommitStems(operation, activation) => {
                self.check_reference(&operation.token)?;
                ensure!(
                    self.managed_generation == operation.generation,
                    "Library changed; discard stale stems"
                );
                let key = self.song.as_ref().unwrap().key.clone();
                ensure!(
                    crate::stems::source::freshness(&operation.reference()?)?
                        == operation.reference_freshness,
                    "Reference changed before publication"
                );
                let current = operation.library.record(&key)?;
                ensure!(
                    current.stems.as_ref().map(|s| &s.id)
                        == operation.record.stems.as_ref().map(|s| &s.id),
                    "Stem set changed; discard stale attachment"
                );
                ensure!(
                    current.record.reference_revision == operation.record.record.reference_revision,
                    "Reference changed; discard stale stems"
                );
                operation.library.save_stems(
                    &key,
                    current.record.reference_revision,
                    Some(activation.record.clone()),
                )?;
                self.stems = Some(activation.clone());
                // Close old readers before deleting their owned storage.
                self.connect();
                if let Some(old) = &current.stems
                    && let Err(e) = operation.library.cleanup_stem_directory(&key, &old.id)
                {
                    self.stems.as_mut().unwrap().warning =
                        Some(format!("Stems attached, but old set cleanup failed: {e:#}"));
                }
                self.stems_busy = false;
            }
            Command::RemoveStems(token, id) => {
                self.check_reference(token)?;
                let old = self.stems.as_ref().context("No stems attached")?;
                ensure!(
                    &old.record.id == id,
                    "Stem set changed; reopen removal choices"
                );
                if let Some(player) = &self.player {
                    player.pause();
                }
                self.persist_position();
                ensure!(
                    self.save_error.is_none(),
                    "Practice settings could not be saved; stems were retained"
                );
                let key = self.song.as_ref().unwrap().key.clone();
                let library = self.managed.clone().context("Library unavailable")?;
                let revision = library.record(&key)?.record.reference_revision;
                library.save_stems(&key, revision, None)?;
                self.stems = None;
                self.connect();
                if let Err(e) = library.cleanup_stem_directory(&key, id) {
                    self.library_error = Some(format!(
                        "Stems removed, but files could not be deleted: {e:#}"
                    ));
                }
            }
            Command::MixStems(token, id, tracks, persist) => {
                self.check_reference(token)?;
                let stems = self.stems.as_mut().context("No stems attached")?;
                ensure!(
                    &stems.record.id == id && stems.prepared.is_some(),
                    "Stem set changed or unavailable"
                );
                let roles = stems.record.roles();
                ensure!(
                    tracks.len() == roles.len(),
                    "Mix must specify each present track exactly once"
                );
                let mut seen = std::collections::HashSet::new();
                let mut mix = MixState::default();
                for track in tracks {
                    let role = Role::parse(&track.role)?;
                    ensure!(
                        roles.contains(&role) && seen.insert(role.index()),
                        "Unknown or repeated mix role"
                    );
                    mix.tracks[role.index()] = crate::stems::RoleControl {
                        gain: track.gain,
                        mute: track.mute,
                        solo: track.solo,
                    };
                }
                mix.validate()?;
                let changed = roles.iter().any(|role| {
                    let a = stems.mix.tracks[role.index()];
                    let b = mix.tracks[role.index()];
                    a.gain != b.gain || a.mute != b.mute || a.solo != b.solo
                });
                if changed && let Some(player) = &self.player {
                    player.set_mix(stems.record.selection(mix), mix)?;
                }
                stems.mix = mix;
                if *persist {
                    for role in roles {
                        let c = mix.tracks[role.index()];
                        stems.record.mix.tracks[role.index()].gain = c.gain;
                        stems.record.mix.tracks[role.index()].mute = c.mute;
                    }
                    let song = self.song.as_ref().unwrap();
                    self.managed
                        .as_ref()
                        .context("Library unavailable")?
                        .save_stems(
                            &song.key,
                            song.reference_revision,
                            Some(stems.record.clone()),
                        )?;
                }
            }
            _ => return Ok(None),
        }
        Ok(Some(Response::StemStatus(self.stem_status())))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        audio::write_wav,
        managed::{ManagedLibrary, PreferenceData, Preferences},
        stems::library::{SavedMix, StoredTrack},
    };
    use std::fs;
    struct Fixture {
        root: PathBuf,
        controller: Controller,
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            self.controller.player = None;
            self.controller.stems = None;
            let _ = fs::remove_dir_all(&self.root);
        }
    }
    fn fixture() -> Fixture {
        let root =
            std::env::temp_dir().join(format!("ohmyloop-desktop-stem-{}", uuid::Uuid::new_v4()));
        fs::create_dir(&root).unwrap();
        let root = root.canonicalize().unwrap();
        let input = root.join("reference.wav");
        let audio = Audio::tone(44100, 1., 440.);
        write_wav(&input, audio.rate, &audio.frames).unwrap();
        let library = ManagedLibrary::create(&root).unwrap();
        let prepared = library
            .commit_import(library.import(&input, |_, _| {}).unwrap())
            .unwrap();
        let record = prepared.record;
        let key = record.id.to_string();
        let id = uuid::Uuid::new_v4().to_string();
        let owned = library.stem_directory(&key, &id).unwrap();
        fs::create_dir_all(&owned).unwrap();
        let file = owned.join("bass.wav");
        fs::copy(&input, &file).unwrap();
        let stored = StoredStemSet {
            version: 1,
            id,
            reference_sha256: record.asset.digest.clone(),
            reference_rate: audio.rate,
            reference_frames: audio.frames.len() as u64,
            provenance: serde_json::json!({"settings":{"normalization":"none per stem"},"model_lock":{"version":1,"model":"demucs-6s","files":{"5c90dfd2-34c22ccb.th":{"sha256":format!("34c22ccb{}","0".repeat(56)),"bytes":1,"url":"https://example.test/test-weights"},"htdemucs_6s.yaml":{"sha256":"0".repeat(64),"bytes":1,"url":"https://example.test/test-config"}}},"preparation":{"version":1,"preprocessing":"symphonia-stereo-rubato-delay-compensated-v1","geometry_policy":"floor(seconds * source_rate); floor(span_frames * 44100 / source_rate); no context","source_sha256":record.asset.digest,"source_rate":audio.rate,"source_frames":audio.frames.len(),"source_span":{"start":0,"end_exclusive":audio.frames.len()},"prepared_rate":audio.rate,"prepared_frames":audio.frames.len(),"channels":2,"prepared_sha256":record.asset.digest}}),
            tracks: vec![StoredTrack {
                role: "bass".into(),
                file: "bass.wav".into(),
                sha256: record.asset.digest.clone(),
                rate: audio.rate,
                frames: audio.frames.len() as u64,
                channels: 2,
            }],
            mix: SavedMix::default(),
        };
        library
            .save_stems(&key, record.reference_revision, Some(stored.clone()))
            .unwrap();
        let factory = Arc::new(
            crate::stems::library::prepare(
                &stored,
                &input,
                &owned,
                &owned.join("playback-cache"),
                audio.rate,
                |_| {},
            )
            .unwrap(),
        );
        let session = SessionId::new();
        let document = record.document(session).unwrap();
        let song = Song {
            key: key.clone(),
            asset_id: record.asset.id,
            timeline_id: record.timeline.id,
            reference_revision: record.reference_revision,
            session_id: session,
            reference_token: record.reference(session).token(),
            title: "fixture".into(),
            path: Some(
                library
                    .asset_path(&record)
                    .unwrap()
                    .to_string_lossy()
                    .into_owned(),
            ),
            duration: audio.duration(),
            rate: audio.rate,
            peaks: vec![],
        };
        let controller = Controller {
            preferences: Preferences::new(&root),
            preference_data: PreferenceData {
                active: Some(crate::managed::LibraryPointer {
                    root: library.root().to_string_lossy().into_owned(),
                    id: library.id(),
                }),
                last_song: Some(key),
                ..Default::default()
            },
            managed: Some(library),
            root_error: None,
            managed_generation: 1,
            activation_generation: 0,
            store: Store::new(root.join("appdata")).unwrap(),
            library: crate::library::Store::new(&root),
            library_error: None,
            song: Some(song),
            audio: Some(Arc::new(audio)),
            document: Some(document),
            player: Some(Player::headless(factory.clone(), true, 44100).unwrap()),
            audio_error: None,
            save_error: None,
            last_save: Instant::now(),
            review_map: None,
            metronome_enabled: false,
            metronome_volume: 0.5,
            count_in: CountInSettings::default(),
            stems: Some(Activation {
                record: stored,
                prepared: Some(factory),
                mix: MixState::default(),
                warning: None,
            }),
            stems_busy: false,
            separation_busy: false,
        };
        Fixture { root, controller }
    }
    fn update(
        f: &mut Fixture,
        gain: f32,
        mute: bool,
        solo: bool,
        persist: bool,
    ) -> Result<Response> {
        let c = &mut f.controller;
        let token = c.song.as_ref().unwrap().reference_token.clone();
        let id = c.stems.as_ref().unwrap().record.id.clone();
        c.request(Command::MixStems(
            token,
            id,
            vec![TrackControl {
                role: "bass".into(),
                gain,
                mute,
                solo,
            }],
            persist,
        ))
    }
    #[test]
    fn live_mix_preserves_transport_and_only_persists_gain_mute() {
        let mut f = fixture();
        let c = &mut f.controller;
        let player = c.player.as_ref().unwrap();
        player.seek(0.25).unwrap();
        let position = c.transport().position;
        let token = c.song.as_ref().unwrap().reference_token.clone();
        update(&mut f, 0.4, true, true, false).unwrap();
        assert_eq!(f.controller.stems.as_ref().unwrap().selection(), 1);
        assert!((f.controller.transport().position - position).abs() < 0.01);
        assert!(!f.controller.transport().playing);
        let key = &f.controller.song.as_ref().unwrap().key;
        assert_eq!(
            f.controller
                .managed
                .as_ref()
                .unwrap()
                .record(key)
                .unwrap()
                .stems
                .unwrap()
                .mix
                .tracks[0]
                .gain,
            1.
        );
        update(&mut f, 0.4, true, true, true).unwrap();
        let restored = f
            .controller
            .managed
            .as_ref()
            .unwrap()
            .record(&f.controller.song.as_ref().unwrap().key)
            .unwrap()
            .stems
            .unwrap()
            .mix
            .to_mix();
        assert_eq!(restored.tracks[0].gain, 0.4);
        assert!(restored.tracks[0].mute);
        assert!(!restored.tracks[0].solo);
        update(&mut f, 1., false, false, false).unwrap();
        assert_eq!(f.controller.stems.as_ref().unwrap().selection(), 0);
        assert_eq!(f.controller.song.as_ref().unwrap().reference_token, token);
    }
    #[test]
    fn rejects_stale_and_invalid_updates_without_mutating_mix() {
        let mut f = fixture();
        let token = f.controller.song.as_ref().unwrap().reference_token.clone();
        let id = f.controller.stems.as_ref().unwrap().record.id.clone();
        assert!(update(&mut f, 2., false, false, false).is_err());
        assert_eq!(f.controller.stems.as_ref().unwrap().mix.tracks[0].gain, 1.);
        assert!(
            f.controller
                .request(Command::MixStems(
                    token.clone(),
                    "stale".into(),
                    vec![],
                    false
                ))
                .is_err()
        );
        assert!(
            f.controller
                .request(Command::MixStems(
                    token.clone(),
                    id,
                    vec![TrackControl {
                        role: "drums".into(),
                        gain: 0.3,
                        mute: false,
                        solo: false
                    }],
                    false
                ))
                .is_err()
        );
        let other = f
            .controller
            .managed
            .as_ref()
            .unwrap()
            .record(&f.controller.song.as_ref().unwrap().key)
            .unwrap()
            .record
            .reference(SessionId::new())
            .token();
        assert!(f.controller.request(Command::StemStatus(other)).is_err());
        assert!(matches!(
            f.controller.request(Command::BeginStems(token)).unwrap(),
            Response::StemOperation(_)
        ));
        assert!(f.controller.request(Command::Load(DEMO.into())).is_err());
        assert!(
            f.controller
                .request(Command::DeleteManaged(
                    f.controller.song.as_ref().unwrap().key.clone()
                ))
                .is_err()
        );
        f.controller.request(Command::FinishStems).unwrap();
    }
    #[test]
    #[ignore = "requires a physical audio output device"]
    fn desktop_stems_hardware_smoke() {
        let mut f = fixture();
        f.controller.player = None;
        let record = f.controller.stems.as_ref().unwrap().record.clone();
        f.controller.stems.as_mut().unwrap().prepared = None;
        let library = f.controller.managed.as_ref().unwrap();
        let reference = PathBuf::from(f.controller.song.as_ref().unwrap().path.as_ref().unwrap());
        let owned = library
            .stem_directory(&f.controller.song.as_ref().unwrap().key, &record.id)
            .unwrap();
        let rate = Player::output_rate().unwrap();
        let factory = Arc::new(
            crate::stems::library::prepare(
                &record,
                &reference,
                &owned,
                &owned.join("playback-cache"),
                rate,
                |_| {},
            )
            .unwrap(),
        );
        f.controller.stems.as_mut().unwrap().prepared = Some(factory);
        f.controller.connect();
        assert!(
            f.controller.audio_error.is_none(),
            "{:?}",
            f.controller.audio_error
        );
        f.controller.action(Action::TogglePlay).unwrap();
        thread::sleep(Duration::from_millis(150));
        update(&mut f, 0.5, false, false, false).unwrap();
        thread::sleep(Duration::from_millis(150));
        update(&mut f, 1., false, false, false).unwrap();
        thread::sleep(Duration::from_millis(150));
        f.controller.action(Action::Pause).unwrap();
        let status = f.controller.transport();
        assert!(status.audio_error.is_none(), "{:?}", status.audio_error);
        let player = f.controller.player.as_ref().unwrap();
        let shared = &player.shared;
        println!(
            "desktop hardware device={} rate={} position={} callbacks={} underflow_frames={} device_errors={} clipped_samples={}",
            player.device_name,
            rate,
            status.position,
            shared.callbacks.load(Ordering::Relaxed),
            shared.underflow_frames.load(Ordering::Relaxed),
            shared.device_errors.load(Ordering::Relaxed),
            shared.clipped_samples.load(Ordering::Relaxed)
        );
        assert!(shared.callbacks.load(Ordering::Relaxed) > 0);
        assert_eq!(shared.device_errors.load(Ordering::Relaxed), 0);
        assert_eq!(shared.clipped_samples.load(Ordering::Relaxed), 0);
        assert!(status.position > 0.2);
    }
    #[test]
    fn late_song_open_cannot_override_newer_open_or_stem_operation() {
        let mut f = fixture();
        let key = f.controller.song.as_ref().unwrap().key.clone();
        let first = match f.controller.request(Command::BeginManagedOpen).unwrap() {
            Response::ManagedOpenContext(lib, generation, opening) => (lib, generation, opening),
            _ => unreachable!(),
        };
        let prepared = first.0.open_song(&key).unwrap();
        f.controller.request(Command::BeginManagedOpen).unwrap();
        assert!(
            f.controller
                .request(Command::ActivateManaged(
                    first.1,
                    first.2,
                    PreparedOpen {
                        recording: prepared.clone(),
                        stems: None
                    }
                ))
                .is_err()
        );
        let active = match f.controller.request(Command::BeginManagedOpen).unwrap() {
            Response::ManagedOpenContext(_, generation, opening) => (generation, opening),
            _ => unreachable!(),
        };
        let token = f.controller.song.as_ref().unwrap().reference_token.clone();
        f.controller.request(Command::BeginStems(token)).unwrap();
        f.controller.request(Command::FinishStems).unwrap();
        assert!(
            f.controller
                .request(Command::ActivateManaged(
                    active.0,
                    active.1,
                    PreparedOpen {
                        recording: prepared,
                        stems: None
                    }
                ))
                .is_err()
        );
    }
    #[test]
    fn unavailable_stems_preserve_requested_settings_and_stale_publication_preserves_record() {
        let mut f = fixture();
        update(&mut f, 0.3, true, false, true).unwrap();
        let c = &mut f.controller;
        let token = c.song.as_ref().unwrap().reference_token.clone();
        let operation = match c.request(Command::BeginStems(token)).unwrap() {
            Response::StemOperation(op) => *op,
            _ => unreachable!(),
        };
        let key = c.song.as_ref().unwrap().key.clone();
        let library = c.managed.clone().unwrap();
        let before = fs::read(library.root().join(format!("songs/{key}/song.json"))).unwrap();
        let activation = c.stems.clone().unwrap();
        c.managed_generation += 1;
        assert!(
            c.request(Command::CommitStems(operation, activation))
                .is_err()
        );
        assert_eq!(
            before,
            fs::read(library.root().join(format!("songs/{key}/song.json"))).unwrap()
        );
        c.request(Command::FinishStems).unwrap();
        c.player = None;
        c.stems.as_mut().unwrap().prepared = None;
        let stored = library.record(&key).unwrap().stems.unwrap();
        fs::remove_file(
            library
                .stem_directory(&key, &stored.id)
                .unwrap()
                .join(&stored.tracks[0].file),
        )
        .unwrap();
        let prepared = library.open_song(&key).unwrap();
        let unavailable = Desktop::prepare_stem_open(&library, prepared, |_| {})
            .stems
            .unwrap();
        assert!(unavailable.prepared.is_none());
        assert!(unavailable.warning.unwrap().contains("Original"));
        assert_eq!(unavailable.mix.tracks[0].gain, 0.3);
        assert!(unavailable.mix.tracks[0].mute);
    }
    #[test]
    fn separation_freezes_transport_but_preserves_note_map_updates_and_rechecks_reference() {
        let mut f = fixture();
        let c = &mut f.controller;
        let token = c.song.as_ref().unwrap().reference_token.clone();
        let operation = match c.request(Command::BeginSeparation(token.clone())).unwrap() {
            Response::StemOperation(op) => *op,
            _ => unreachable!(),
        };
        assert!(
            c.request(Command::Act(Action::Seek { seconds: 1. }, token.clone()))
                .is_err()
        );
        assert!(
            c.request(Command::MixStems(
                token.clone(),
                c.stems.as_ref().unwrap().record.id.clone(),
                vec![],
                false
            ))
            .is_err()
        );
        assert!(c.request(Command::BeginManagedOpen).is_err());
        let mut document = c.document.clone().unwrap();
        document.notes = "written during separation".into();
        c.request(Command::Update(document.clone())).unwrap();
        document.controls.speed = 0.5;
        assert!(c.request(Command::Update(document)).is_err());
        let activation = c.stems.clone().unwrap();
        let reference = operation.reference().unwrap();
        let mut bytes = fs::read(&reference).unwrap();
        bytes.push(0);
        fs::write(reference, bytes).unwrap();
        assert!(
            c.request(Command::CommitStems(operation, activation))
                .is_err()
        );
        assert_eq!(
            c.document.as_ref().unwrap().notes,
            "written during separation"
        );
        c.request(Command::FinishSeparation).unwrap();
        assert!(!c.separation_busy);
    }
    #[test]
    fn successful_generated_publication_keeps_authored_map_notes_position_and_controls() {
        let mut f = fixture();
        update(&mut f, 0.4, true, true, true).unwrap();
        let c = &mut f.controller;
        let token = c.song.as_ref().unwrap().reference_token.clone();
        let mut document = c.document.clone().unwrap();
        document.controls.speed = 0.75;
        document.controls.semitones = -2.;
        c.request(Command::Update(document)).unwrap();
        c.action(Action::Seek { seconds: 0.3 }).unwrap();
        let operation = match c.request(Command::BeginSeparation(token.clone())).unwrap() {
            Response::StemOperation(op) => *op,
            _ => unreachable!(),
        };
        let mut document = c.document.clone().unwrap();
        let map = crate::timing::MeasureMap::empty(44100, 44100)
            .edit(crate::timing::Edit::Build {
                start: 0,
                end: 44100,
                count: 2,
                meter: crate::timing::Meter::common(),
            })
            .unwrap()
            .map;
        document.measure_map = Some(map.clone());
        document.notes = "authored while separating".into();
        c.request(Command::Update(document)).unwrap();
        let owned = operation.owned(&uuid::Uuid::new_v4().to_string()).unwrap();
        fs::create_dir_all(&owned).unwrap();
        let reference = operation.reference().unwrap();
        fs::copy(&reference, owned.join("prepared.wav")).unwrap();
        let identity = operation.record.stems.as_ref().unwrap().provenance.clone();
        let stems=crate::stems::Role::ALL.into_iter().map(|role|{let name=role.name();fs::copy(&reference,owned.join(format!("{name}.wav"))).unwrap();(name.to_owned(),serde_json::json!({"file":format!("{name}.wav"),"sha256":operation.digest(),"frames":44100,"rate":44100,"channels":2}))}).collect::<serde_json::Map<_,_>>();
        use sha2::Digest;
        let manifest = serde_json::json!({"version":1,"status":"complete","job_key":format!("{:x}",sha2::Sha256::digest(serde_json::to_vec(&identity).unwrap())),"identity":identity,"corrections":[],"stems":stems});
        fs::write(
            owned.join("manifest.json"),
            serde_json::to_vec(&manifest).unwrap(),
        )
        .unwrap();
        let (mut record, prepared) = crate::stems::library::attach_generated(
            &reference,
            &owned,
            44100,
            &crate::stems::WorkControl::default(),
            |_| {},
        )
        .unwrap();
        for role in record.roles() {
            if let Some(old) = &operation.record.stems
                && old.roles().contains(&role)
            {
                record.mix.tracks[role.index()] = old.mix.tracks[role.index()].clone();
            }
        }
        let mix = record.mix.to_mix();
        c.request(Command::CommitStems(
            operation.clone(),
            Activation {
                record: record.clone(),
                prepared: Some(Arc::new(prepared)),
                mix,
                warning: None,
            },
        ))
        .unwrap();
        c.request(Command::FinishSeparation).unwrap();
        let current = operation.library.record(&record_key(&operation)).unwrap();
        assert_eq!(current.record.notes, "authored while separating");
        assert_eq!(current.record.timeline.measure_map, Some(map));
        assert_eq!(current.record.controls.speed, 0.75);
        assert_eq!(current.record.controls.semitones, -2.);
        assert!((c.transport().position - 0.3).abs() < 0.001);
        assert!(!c.transport().playing);
        assert_eq!(current.stems.as_ref().unwrap().mix.tracks[0].gain, 0.4);
        assert!(current.stems.as_ref().unwrap().mix.tracks[0].mute);
        assert!(!c.stems.as_ref().unwrap().mix.tracks[0].solo);
        assert_eq!(current.stems.unwrap().mix.tracks[1].gain, 1.);
    }
    fn record_key(operation: &Operation) -> String {
        operation.record.record.id.to_string()
    }
}
