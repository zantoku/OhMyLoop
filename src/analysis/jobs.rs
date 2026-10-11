use super::{
    models,
    progress::{ProgressUnit, ProgressUpdate, WorkProgress, read_updates},
    worker::{Candidates, read_result},
};
use crate::{analysis_pcm::write_model_pcm_with_progress, audio::Audio};
use anyhow::{Context, Result, ensure};
use serde::Serialize;
use std::{
    fs,
    io::Read,
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    thread,
    time::{Duration, Instant},
};
fn valid_key(key: &str) -> bool {
    !key.is_empty()
        && key.len() <= 128
        && key.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'-')
}
#[derive(Clone, Serialize)]
pub struct JobStatus {
    pub id: u64,
    pub running: bool,
    pub kind: String,
    pub phase: String,
    pub model_ready: bool,
    pub model_path: String,
    pub progress: Option<WorkProgress>,
    pub elapsed_seconds: f64,
    pub song_key: Option<String>,
    pub map_revision: u64,
    pub error: Option<String>,
    pub result: Option<Candidates>,
}
struct Inner {
    state: Mutex<JobStatus>,
    cancel: AtomicBool,
    child: Mutex<Option<Child>>,
    models: PathBuf,
    cache: PathBuf,
    started: Mutex<Instant>,
}
#[derive(Clone)]
pub struct AnalysisManager(Arc<Inner>);
impl AnalysisManager {
    pub fn new(models: PathBuf, cache: PathBuf) -> Result<Self> {
        fs::create_dir_all(&cache)?;
        let models = std::path::absolute(models)?;
        let model_path = models.to_string_lossy().into_owned();
        let model_ready = models::present(&models);
        Ok(Self(Arc::new(Inner {
            started: Mutex::new(Instant::now()),
            state: Mutex::new(JobStatus {
                id: 0,
                running: false,
                kind: "idle".into(),
                phase: "Ready".into(),
                model_ready,
                model_path,
                progress: None,
                elapsed_seconds: 0.0,
                song_key: None,
                map_revision: 0,
                error: None,
                result: None,
            }),
            cancel: AtomicBool::new(false),
            child: Mutex::new(None),
            models,
            cache,
        })))
    }
    pub fn status(&self) -> JobStatus {
        let mut status = self.0.state.lock().unwrap().clone();
        if status.running {
            status.elapsed_seconds = self.0.started.lock().unwrap().elapsed().as_secs_f64();
        }
        status
    }
    fn begin(&self, kind: &str, key: Option<String>, revision: u64) -> Result<u64> {
        let mut state = self.0.state.lock().unwrap();
        ensure!(
            !state.running,
            "An analysis or model installation is already running"
        );
        state.id += 1;
        state.running = true;
        state.kind = kind.into();
        state.phase = "Starting".into();
        state.progress = None;
        state.elapsed_seconds = 0.0;
        *self.0.started.lock().unwrap() = Instant::now();
        state.song_key = key;
        state.map_revision = revision;
        state.error = None;
        state.result = None;
        self.0.cancel.store(false, Ordering::Release);
        Ok(state.id)
    }
    fn phase(&self, phase: &str) {
        let mut state = self.0.state.lock().unwrap();
        if !self.0.cancel.load(Ordering::Acquire) {
            state.phase = phase.into();
            state.progress = None;
        }
    }
    fn progress(&self, id: u64, update: ProgressUpdate) {
        let mut state = self.0.state.lock().unwrap();
        if state.id == id && state.running && !self.0.cancel.load(Ordering::Acquire) {
            state.phase = update.phase;
            state.progress = update.progress;
        }
    }
    fn finish(&self, result: Result<Option<Candidates>>) {
        let mut state = self.0.state.lock().unwrap();
        state.running = false;
        state.progress = None;
        state.elapsed_seconds = self.0.started.lock().unwrap().elapsed().as_secs_f64();
        if self.0.cancel.load(Ordering::Acquire) {
            state.phase = "Cancelled".into();
            state.result = None;
            state.error = None;
        } else {
            match result {
                Ok(candidates) => {
                    state.phase = "Complete".into();
                    state.result = candidates;
                    state.error = None;
                }
                Err(e) => {
                    state.phase = "Failed".into();
                    state.error = Some(format!("{e:#}"));
                    state.result = None;
                }
            }
        }
        state.model_ready = models::present(&self.0.models);
    }
    pub fn install(&self) -> Result<JobStatus> {
        let id = self.begin("install", None, 0)?;
        let manager = self.clone();
        if let Err(e) = thread::Builder::new()
            .name("ohmyloop-model-install".into())
            .spawn(move || {
                let result = models::install(&manager.0.models, &manager.0.cancel, |update| {
                    manager.progress(id, update)
                })
                .map(|()| None);
                manager.finish(result);
            })
        {
            self.finish(Err(e.into()));
        }
        Ok(self.status())
    }
    /// Reload suggestions independently of job state and accepted musical edits.
    pub fn cached_result(&self, key: &str, audio: &Audio) -> Result<Option<Candidates>> {
        ensure!(valid_key(key), "Invalid analysis song identity");
        let path = self.0.cache.join(format!(
            "{}-{}-{}.json",
            key,
            models::MODEL_ID,
            crate::analysis_pcm::PREPROCESSING
        ));
        match fs::metadata(&path) {
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(e) => return Err(e.into()),
            Ok(_) => {}
        }
        let result = read_result(&path)?;
        ensure!(
            result.song_key == key
                && result.rate == audio.rate
                && result.frames == audio.frames.len() as u64,
            "Cached analysis belongs to another source"
        );
        Ok(Some(result))
    }
    pub fn worker_pid(&self) -> Option<u32> {
        self.0.child.lock().unwrap().as_ref().map(Child::id)
    }
    pub fn analyze(
        &self,
        executable: PathBuf,
        audio: Arc<Audio>,
        key: String,
        revision: u64,
    ) -> Result<JobStatus> {
        ensure!(valid_key(&key), "Invalid analysis song identity");
        ensure!(
            audio.duration() <= 3600.0,
            "Automatic analysis is limited to one hour; manual mapping remains available"
        );
        ensure!(
            models::present(&self.0.models),
            "Install the analysis model first. Manual mapping remains available."
        );
        let id = self.begin("analysis", Some(key.clone()), revision)?;
        let manager = self.clone();
        if let Err(e) = thread::Builder::new()
            .name("ohmyloop-analysis".into())
            .spawn(move || {
                let directory = manager.0.cache.join(format!(
                    "job-{}-{id}-{}",
                    std::process::id(),
                    std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_nanos()
                ));
                let input = directory.join("source.wav");
                let output = directory.join("detections.json");
                let errors = directory.join("worker-error.txt");
                let mut progress_reader = None;
                let result = (|| -> Result<Option<Candidates>> {
                    fs::create_dir(&directory).context("Could not create analysis job cache")?;
                    manager.phase("Verifying model files");
                    models::verify_models(&manager.0.models)?;
                    manager.phase("Preparing source-aligned audio");
                    write_model_pcm_with_progress(
                        &audio,
                        &input,
                        &manager.0.cancel,
                        |completed, total| {
                            manager.progress(
                                id,
                                ProgressUpdate::measured(
                                    "Preparing audio",
                                    completed,
                                    Some(total),
                                    ProgressUnit::Frames,
                                ),
                            );
                        },
                    )?;
                    ensure!(
                        !manager.0.cancel.load(Ordering::Acquire),
                        "Analysis cancelled"
                    );
                    manager.phase("Analyzing beats and downbeats");
                    let mut command = Command::new(&executable);
                    command
                        .arg("--analysis-worker")
                        .arg(&input)
                        .arg(&output)
                        .arg(&manager.0.models)
                        .arg(&key)
                        .arg(audio.rate.to_string())
                        .arg(audio.frames.len().to_string())
                        .arg(revision.to_string())
                        .env("RAYON_NUM_THREADS", "2")
                        .env("RTEN_NUM_THREADS", "2")
                        .stdin(Stdio::null())
                        .stdout(Stdio::piped())
                        .stderr(Stdio::from(fs::File::create(&errors)?));
                    #[cfg(target_os = "windows")]
                    {
                        use std::os::windows::process::CommandExt;
                        command.creation_flags(0x08000000);
                    }
                    // Hold the child slot across spawn so cancellation cannot miss it.
                    {
                        let mut slot = manager.0.child.lock().unwrap();
                        ensure!(
                            !manager.0.cancel.load(Ordering::Acquire),
                            "Analysis cancelled"
                        );
                        *slot = Some(
                            command
                                .spawn()
                                .context("Could not start the analysis process")?,
                        );
                    }
                    let stdout = manager
                        .0
                        .child
                        .lock()
                        .unwrap()
                        .as_mut()
                        .context("Analysis process disappeared")?
                        .stdout
                        .take()
                        .context("Worker progress pipe is missing")?;
                    let observer = manager.clone();
                    progress_reader = Some(
                        thread::Builder::new()
                            .name("ohmyloop-analysis-progress".into())
                            .spawn(move || {
                                read_updates(stdout, |update| observer.progress(id, update));
                            })?,
                    );
                    loop {
                        let status = {
                            let mut slot = manager.0.child.lock().unwrap();
                            let child = slot.as_mut().context("Analysis process disappeared")?;
                            if manager.0.cancel.load(Ordering::Acquire) {
                                let _ = child.kill();
                            }
                            child.try_wait()?
                        };
                        if let Some(status) = status {
                            manager.0.child.lock().unwrap().take();
                            if !status.success() {
                                let mut message = String::new();
                                fs::File::open(&errors)?
                                    .take(16 * 1024)
                                    .read_to_string(&mut message)?;
                                anyhow::bail!("Beat analysis stopped: {message}");
                            }
                            break;
                        }
                        thread::sleep(Duration::from_millis(25));
                    }
                    ensure!(
                        !manager.0.cancel.load(Ordering::Acquire),
                        "Analysis cancelled"
                    );
                    manager.phase("Reading detected beats");
                    let result = read_result(&output)?;
                    ensure!(
                        result.song_key == key
                            && result.rate == audio.rate
                            && result.frames == audio.frames.len() as u64
                            && result.map_revision == revision,
                        "Analysis result belongs to another source"
                    );
                    Ok(Some(result))
                })();
                // Always reap our subprocess, including errors in monitoring/reading.
                if let Some(mut child) = manager.0.child.lock().unwrap().take() {
                    let _ = child.kill();
                    let _ = child.wait();
                }
                if let Some(reader) = progress_reader {
                    let _ = reader.join();
                }
                let _ = fs::remove_file(input);
                let _ = fs::remove_file(errors);
                if result.is_ok() && !manager.0.cancel.load(Ordering::Acquire) {
                    let stable = manager.0.cache.join(format!(
                        "{}-{}-{}.json",
                        key,
                        models::MODEL_ID,
                        crate::analysis_pcm::PREPROCESSING
                    ));
                    if fs::rename(&output, stable).is_ok() {
                        let _ = fs::remove_dir(&directory);
                    }
                }
                if result.is_err() || manager.0.cancel.load(Ordering::Acquire) {
                    let _ = fs::remove_file(output);
                    let _ = fs::remove_dir(&directory);
                }
                manager.finish(result);
            })
        {
            self.finish(Err(e.into()));
        }
        Ok(self.status())
    }
    pub fn cancel(&self, id: u64) -> JobStatus {
        let mut state = self.0.state.lock().unwrap();
        if state.id == id && state.running {
            self.0.cancel.store(true, Ordering::Release);
            state.phase = "Cancelling".into();
            state.progress = None;
            if let Some(child) = self.0.child.lock().unwrap().as_mut() {
                let _ = child.kill();
            }
        }
        drop(state);
        self.status()
    }
    pub fn shutdown(&self) {
        let id = self.status().id;
        self.cancel(id);
        if let Some(mut child) = self.0.child.lock().unwrap().take() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
    pub fn model_directory(&self) -> &Path {
        &self.0.models
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cached_suggestions_restore_without_models_and_validate_source_preserving_invalid_files() {
        let root =
            std::env::temp_dir().join(format!("ohmyloop-analysis-cache-{}", std::process::id()));
        let cache = root.join("cache");
        let manager = AnalysisManager::new(root.join("missing-models"), cache.clone()).unwrap();
        let audio = Audio::tone(1000, 2.0, 220.0);
        assert!(manager.cached_result("song", &audio).unwrap().is_none());
        assert!(manager.cached_result("../song", &audio).is_err());
        let path = cache.join(format!(
            "song-{}-{}.json",
            models::MODEL_ID,
            crate::analysis_pcm::PREPROCESSING
        ));
        let valid = serde_json::json!({"candidates": {
            "version":1,"song_key":"song","map_revision":8,"rate":audio.rate,"frames":audio.frames.len(),
            "model":models::MODEL_ID,"preprocessing":crate::analysis_pcm::PREPROCESSING,"elapsed_seconds":1.0,
            "beats":[{"frame":0,"score":0.9,"downbeat":true},{"frame":1000,"score":0.8,"downbeat":false}]
        },"beat_logits":[],"downbeat_logits":[]});
        fs::write(&path, serde_json::to_vec(&valid).unwrap()).unwrap();
        let result = manager.cached_result("song", &audio).unwrap().unwrap();
        assert_eq!(result.map_revision, 8);
        assert_eq!(result.beats.len(), 2);
        assert!(manager.status().result.is_none());
        assert!(!manager.status().running);
        for (field, value) in [
            ("song_key", serde_json::json!("other")),
            ("rate", serde_json::json!(2000)),
            ("frames", serde_json::json!(100)),
            ("version", serde_json::json!(2)),
            ("model", serde_json::json!("other")),
        ] {
            let mut invalid = valid.clone();
            invalid["candidates"][field] = value;
            let bytes = serde_json::to_vec(&invalid).unwrap();
            fs::write(&path, &bytes).unwrap();
            assert!(manager.cached_result("song", &audio).is_err());
            assert_eq!(fs::read(&path).unwrap(), bytes);
        }
        fs::write(&path, b"broken JSON").unwrap();
        assert!(manager.cached_result("song", &audio).is_err());
        assert_eq!(fs::read(&path).unwrap(), b"broken JSON");
        fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn progress_is_bound_to_a_running_job_and_elapsed_time_freezes_at_finish() {
        let root =
            std::env::temp_dir().join(format!("ohmyloop-progress-state-{}", std::process::id()));
        let manager = AnalysisManager::new(root.join("models"), root.join("cache")).unwrap();
        let id = manager.begin("analysis", Some("song".into()), 0).unwrap();
        let update = ProgressUpdate::measured("Finding beats", 1, Some(4), ProgressUnit::Segments);
        manager.progress(id + 1, update.clone());
        assert!(manager.status().progress.is_none());
        manager.progress(id, update.clone());
        assert_eq!(manager.status().progress, update.progress);
        manager.cancel(id);
        manager.progress(id, update.clone());
        assert_eq!(manager.status().phase, "Cancelling");
        manager.finish(Ok(None));
        let elapsed = manager.status().elapsed_seconds;
        thread::sleep(Duration::from_millis(10));
        assert_eq!(manager.status().elapsed_seconds, elapsed);
        let next = manager.begin("analysis", Some("song".into()), 0).unwrap();
        manager.progress(id, update);
        assert!(manager.status().progress.is_none());
        manager.cancel(next);
        manager.finish(Ok(None));
        fs::remove_dir_all(root).unwrap();
    }
}
