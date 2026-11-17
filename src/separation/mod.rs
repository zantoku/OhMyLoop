//! App-lifetime model acquisition and supervised, owned stem generation.
mod command;
pub mod control;
mod disk;
mod jobs;
mod managed;
pub mod package;
mod prepare;
mod process_tree;
pub mod protocol;
pub mod runtime;
mod setup;
pub mod smoke;
mod supervisor;
pub mod timing;
use crate::{
    heavy_work::HeavyWork,
    stems::{PreparationProgress, library::WorkflowTracker},
};
use anyhow::{Result, ensure};
use control::Control;
use serde::Serialize;
use serde_json::Value;
use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
    thread::{self, JoinHandle},
    time::Instant,
};
#[derive(Clone, Serialize)]
pub struct Progress {
    pub stage: String,
    pub completed: u64,
    pub total: Option<u64>,
    pub workflow: Option<crate::stems::library::WorkflowProgress>,
}
#[derive(Clone, Serialize)]
pub struct Attempt {
    pub order: u64,
    pub id: String,
    pub running: bool,
    pub phase: String,
    pub reference_token: Option<String>,
    pub progress: Option<Progress>,
    pub error: Option<String>,
    pub details: String,
    pub elapsed_seconds: f64,
    pub cancellable: bool,
    pub result: Option<Value>,
}
impl Default for Attempt {
    fn default() -> Self {
        Self {
            order: 0,
            id: String::new(),
            running: false,
            phase: "Idle".into(),
            reference_token: None,
            progress: None,
            error: None,
            details: String::new(),
            elapsed_seconds: 0.,
            cancellable: false,
            result: None,
        }
    }
}
#[derive(Clone, Serialize)]
pub struct Status {
    pub package_id: String,
    pub profile_id: String,
    pub model_ready: bool,
    pub model_path: String,
    pub package_bytes: u64,
    pub runtime: runtime::RuntimeStatus,
    pub heavy_work: Option<String>,
    pub install: Attempt,
    pub job: Attempt,
}
struct State {
    shutting_down: bool,
    workflow: Option<WorkflowTracker>,
    timings: Option<timing::Timings>,
    install: Attempt,
    job: Attempt,
    install_started: Instant,
    job_started: Instant,
    install_control: Control,
    job_control: Control,
    runtime: runtime::RuntimeStatus,
}
struct Inner {
    state: Mutex<State>,
    workers: Mutex<Vec<JoinHandle<()>>>,
    shutdown: Mutex<()>,
    models: PathBuf,
    runtime: runtime::Runtime,
    gate: HeavyWork,
}
#[derive(Clone)]
pub struct SeparationManager(Arc<Inner>);
impl SeparationManager {
    pub fn new(models: PathBuf, gate: HeavyWork) -> Self {
        let runtime = runtime::Runtime::development();
        Self::with_runtime(models, gate, runtime)
    }
    pub fn managed(
        models: PathBuf,
        tools: PathBuf,
        resources: &std::path::Path,
        gate: HeavyWork,
    ) -> Self {
        Self::with_runtime(models, gate, runtime::Runtime::managed(resources, tools))
    }
    fn with_runtime(models: PathBuf, gate: HeavyWork, runtime: runtime::Runtime) -> Self {
        let ready = runtime.status();
        Self(Arc::new(Inner {
            state: Mutex::new(State {
                shutting_down: false,
                workflow: None,
                timings: None,
                install: Attempt::default(),
                job: Attempt::default(),
                install_started: Instant::now(),
                job_started: Instant::now(),
                install_control: Control::default(),
                job_control: Control::default(),
                runtime: ready,
            }),
            workers: Mutex::new(Vec::new()),
            shutdown: Mutex::new(()),
            models: package::path(&models),
            runtime,
            gate,
        }))
    }
    pub fn processing_estimate(
        &self,
        desktop: &crate::desktop::Desktop,
        reference: String,
    ) -> Result<u64> {
        let context = desktop.separation_context(reference)?;
        disk::estimate(
            context.frames(),
            context.rate(),
            crate::playback::Player::output_rate().unwrap_or(context.rate()),
        )
    }
    pub fn status(&self) -> Status {
        let state = self.0.state.lock().unwrap();
        let mut install = state.install.clone();
        let mut job = state.job.clone();
        if install.running {
            install.elapsed_seconds = state.install_started.elapsed().as_secs_f64();
        }
        if job.running {
            job.elapsed_seconds = state.job_started.elapsed().as_secs_f64();
        }
        Status {
            package_id: package::ID.into(),
            profile_id: package::PROFILE.into(),
            model_ready: package::present(&self.0.models),
            model_path: self.0.models.display().to_string(),
            package_bytes: package::descriptor().bytes(),
            runtime: state.runtime.clone(),
            heavy_work: self.0.gate.active().map(str::to_owned),
            install,
            job,
        }
    }
    pub fn refresh_runtime(&self) -> Status {
        let runtime = self.0.runtime.status();
        self.0.state.lock().unwrap().runtime = runtime;
        self.status()
    }
    fn begin(&self, install: bool, reference: Option<String>) -> Result<(String, Control)> {
        let mut s = self.0.state.lock().unwrap();
        ensure!(!s.shutting_down, "Application is shutting down");
        ensure!(
            !s.install.running && !s.job.running,
            "Model or separation work is already running"
        );
        let id = uuid::Uuid::new_v4().to_string();
        let control = Control::default();
        let attempt = Attempt {
            order: s
                .install
                .order
                .max(s.job.order)
                .checked_add(1)
                .ok_or_else(|| anyhow::anyhow!("Attempt sequence exhausted"))?,
            id: id.clone(),
            running: true,
            phase: "Starting".into(),
            reference_token: reference,
            cancellable: true,
            ..Attempt::default()
        };
        if install {
            s.install = attempt;
            s.install_started = Instant::now();
            s.install_control = control.clone();
        } else {
            let began = Instant::now();
            s.timings = Some(timing::Timings::new(began));
            s.workflow = Some(WorkflowTracker::separation());
            s.job = attempt;
            s.job_started = began;
            s.job_control = control.clone();
        }
        Ok((id, control))
    }
    /// Current-attempt in-memory measurements for maintainer probes, not persisted history.
    pub fn timings(&self) -> Vec<timing::StageTiming> {
        self.0
            .state
            .lock()
            .unwrap()
            .timings
            .as_ref()
            .map(|t| t.snapshot())
            .unwrap_or_default()
    }
    fn progress(&self, install: bool, id: &str, phase: &str, mut p: Option<Progress>) {
        let mut s = self.0.state.lock().unwrap();
        if !install
            && s.job.id == id
            && s.job.running
            && let (Some(timings), Some(p)) = (&mut s.timings, &p)
        {
            timings.stage(&p.stage);
        }
        if !install
            && s.job.id == id
            && s.job.running
            && s.job.phase != "Stopping"
            && let Some(p) = &mut p
            && let Some(workflow) = &mut s.workflow
        {
            p.workflow = workflow
                .update(PreparationProgress {
                    phase: phase.into(),
                    completed: p.completed,
                    total: p.total,
                    step: Some(p.stage.clone()),
                    workflow: None,
                })
                .workflow;
        }
        let a = if install { &mut s.install } else { &mut s.job };
        if a.id == id && a.running && a.phase != "Stopping" {
            a.phase = phase.into();
            a.progress = p;
        }
    }
    fn preparation(&self, id: &str, p: PreparationProgress) {
        let phase = match p.step.as_deref().unwrap_or("") {
            "validation.headroom" => "Measuring playback levels",
            "cache.adopt" | "cache.save" => "Preparing playback",
            "attachment.publish" => "Finishing save",
            _ => "Validating tracks",
        };
        self.progress(
            false,
            id,
            phase,
            Some(Progress {
                stage: p.step.unwrap_or_else(|| "validate".into()),
                completed: p.completed,
                total: p.total,
                workflow: p.workflow,
            }),
        );
    }
    fn finish(&self, install: bool, id: &str, control: &Control, result: Result<Option<Value>>) {
        let mut s = self.0.state.lock().unwrap();
        if (if install { &s.install.id } else { &s.job.id }) != id {
            return;
        }
        if !install && let Some(timing) = &mut s.timings {
            timing.finish();
        }
        let elapsed = if install {
            s.install_started.elapsed().as_secs_f64()
        } else {
            s.timings
                .as_ref()
                .map(|t| t.snapshot().iter().map(|s| s.seconds).sum())
                .unwrap_or_else(|| s.job_started.elapsed().as_secs_f64())
        };
        if !install
            && result.is_ok()
            && !control.cancelled()
            && let Some(w) = &mut s.workflow
        {
            let final_workflow = w.finish().workflow;
            if let Some(p) = &mut s.job.progress {
                p.workflow = final_workflow;
            }
        }
        let a = if install { &mut s.install } else { &mut s.job };
        if a.id != id {
            return;
        }
        a.running = false;
        a.cancellable = false;
        a.elapsed_seconds = elapsed;
        if control.cancelled() {
            a.phase = "Cancelled".into();
            a.error = None;
            if let Err(e) = result
                && e.to_string() != "Cancelled"
            {
                a.details = bounded_details(&format!("{e:#}"));
                a.error = Some(e.to_string());
            }
        } else {
            match result {
                Ok(value) => {
                    a.phase = "Complete".into();
                    a.result = value;
                }
                Err(e) => {
                    a.phase = "Failed".into();
                    a.error = Some(e.to_string());
                    let text = format!("{e:#}");
                    a.details = bounded_details(&text);
                }
            }
        }
    }
    fn spawn(&self, name: &str, work: impl FnOnce() + Send + 'static) -> Result<()> {
        let mut workers = self.0.workers.lock().unwrap();
        ensure!(
            !self.0.state.lock().unwrap().shutting_down,
            "Application is shutting down"
        );
        workers.retain(|h| !h.is_finished());
        workers.push(thread::Builder::new().name(name.into()).spawn(work)?);
        Ok(())
    }
    pub fn remove_model(&self) -> Result<Status> {
        let s = self.0.state.lock().unwrap();
        ensure!(!s.install.running && !s.job.running, "Model is in use");
        package::remove(&self.0.models)?;
        drop(s);
        Ok(self.status())
    }
    pub fn cancel(&self, id: &str) -> Status {
        let mut s = self.0.state.lock().unwrap();
        if s.install.id == id && s.install.running && s.install_control.cancel() {
            s.install.phase = "Stopping".into();
            s.install.cancellable = false;
        }
        if s.job.id == id && s.job.running && s.job_control.cancel() {
            s.job.phase = "Stopping".into();
            s.job.cancellable = false;
        }
        drop(s);
        self.status()
    }
    pub fn active(&self) -> bool {
        let s = self.0.state.lock().unwrap();
        s.install.running || s.job.running
    }
    pub fn shutdown(&self) {
        // Every caller waits for the same cleanup, including an already-started save.
        let _shutdown = self.0.shutdown.lock().unwrap();
        let workers = {
            let mut workers = self.0.workers.lock().unwrap();
            let mut state = self.0.state.lock().unwrap();
            state.shutting_down = true;
            if state.install.running && state.install_control.cancel() {
                state.install.phase = "Stopping".into();
                state.install.cancellable = false;
            }
            if state.job.running && state.job_control.cancel() {
                state.job.phase = "Stopping".into();
                state.job.cancellable = false;
            }
            std::mem::take(&mut *workers)
        };
        for worker in workers {
            let _ = worker.join();
        }
    }
}
fn bounded_details(text: &str) -> String {
    let mut start = text.len().saturating_sub(protocol::DIAGNOSTIC_BYTES);
    while !text.is_char_boundary(start) {
        start += 1;
    }
    text[start..].into()
}

#[cfg(test)]
mod shutdown_tests {
    use super::*;
    use std::sync::{
        Barrier,
        atomic::{AtomicBool, Ordering},
    };
    fn manager() -> SeparationManager {
        SeparationManager(Arc::new(Inner {
            state: Mutex::new(State {
                shutting_down: false,
                workflow: None,
                timings: None,
                install: Attempt::default(),
                job: Attempt::default(),
                install_started: Instant::now(),
                job_started: Instant::now(),
                install_control: Control::default(),
                job_control: Control::default(),
                runtime: runtime::RuntimeStatus {
                    ready: false,
                    supported: true,
                    id: None,
                    message: "Fixture needs no Python or model".into(),
                    path: None,
                },
            }),
            workers: Mutex::new(Vec::new()),
            shutdown: Mutex::new(()),
            models: std::env::temp_dir().join(uuid::Uuid::new_v4().to_string()),
            runtime: runtime::Runtime::development(),
            gate: HeavyWork::default(),
        }))
    }
    #[test]
    fn unsuccessful_attempts_keep_workflow_incomplete_and_finalize_timings() {
        for cancelled in [false, true] {
            let manager = manager();
            let (id, control) = manager.begin(false, Some("reference".into())).unwrap();
            manager.measured(&id, "worker.infer", "Separating instruments", 1, Some(2));
            if cancelled {
                control.cancel();
            }
            manager.finish(
                false,
                &id,
                &control,
                Err(anyhow::anyhow!(if cancelled {
                    "Cancelled"
                } else {
                    "Read failed"
                })),
            );
            let terminal = manager.status().job;
            assert!(!terminal.running);
            let workflow = terminal.progress.unwrap().workflow.unwrap();
            assert!(workflow.completed < workflow.total as f64);
            assert_eq!(
                workflow
                    .steps
                    .iter()
                    .find(|s| s.id == "worker.infer")
                    .unwrap()
                    .state,
                "active"
            );
            let timings = manager.timings();
            assert_eq!(timings.last().unwrap().stage, "worker.infer");
            assert_eq!(
                timings.iter().map(|t| t.seconds).sum::<f64>(),
                terminal.elapsed_seconds
            );
            let (retry, _) = manager.begin(false, Some("reference".into())).unwrap();
            manager.measured(&retry, "model.verify", "Verifying model", 0, None);
            assert_eq!(
                manager
                    .status()
                    .job
                    .progress
                    .unwrap()
                    .workflow
                    .unwrap()
                    .completed,
                0.
            );
        }
    }
    #[test]
    fn stale_terminal_result_cannot_complete_a_new_attempt() {
        let manager = manager();
        let (old, old_control) = manager.begin(false, None).unwrap();
        manager.finish(false, &old, &old_control, Ok(None));
        let (current, _) = manager.begin(false, None).unwrap();
        manager.measured(&current, "model.verify", "Verifying model", 0, None);
        manager.finish(false, &old, &old_control, Ok(None));
        let job = manager.status().job;
        assert!(job.running);
        assert_eq!(job.id, current);
        assert_eq!(job.progress.unwrap().workflow.unwrap().completed, 0.);
        assert!(manager.timings().iter().all(|t| t.stage != "model.verify"));
    }
    #[test]
    fn shutdown_serializes_handle_registration_and_rejects_late_admission() {
        for _ in 0..50 {
            let manager = manager();
            let (id, control) = manager.begin(true, None).unwrap();
            let barrier = Arc::new(Barrier::new(2));
            let done = Arc::new(AtomicBool::new(false));
            let observer = manager.clone();
            let start = barrier.clone();
            let finished = done.clone();
            let spawner = thread::spawn(move || {
                start.wait();
                let worker = observer.clone();
                let key = id.clone();
                let token = control.clone();
                let result = observer.spawn("ohmyloop-shutdown-fixture", move || {
                    thread::sleep(std::time::Duration::from_millis(20));
                    finished.store(true, Ordering::Release);
                    worker.finish(true, &key, &token, Ok(None));
                });
                let accepted = result.is_ok();
                if let Err(error) = result {
                    observer.finish(true, &id, &control, Err(error));
                }
                accepted
            });
            barrier.wait();
            manager.shutdown();
            let accepted = spawner.join().unwrap();
            assert!(
                !accepted || done.load(Ordering::Acquire),
                "shutdown returned before its accepted worker finished"
            );
            assert!(manager.begin(true, None).is_err());
            assert!(
                manager
                    .spawn("late-worker", || panic!("closed manager started work"))
                    .is_err()
            );
            assert!(!manager.status().install.running);
        }
    }
    #[test]
    fn shutdown_waits_for_publication_that_won_cancellation() {
        let manager = manager();
        let (id, control) = manager.begin(false, Some("reference".into())).unwrap();
        control.commit().unwrap();
        let done = Arc::new(AtomicBool::new(false));
        let complete = done.clone();
        let worker = manager.clone();
        manager
            .spawn("ohmyloop-commit-fixture", move || {
                thread::sleep(std::time::Duration::from_millis(20));
                complete.store(true, Ordering::Release);
                worker.finish(false, &id, &control, Ok(None));
            })
            .unwrap();
        manager.shutdown();
        assert!(done.load(Ordering::Acquire));
        assert_eq!(manager.status().job.phase, "Complete");
    }
    #[test]
    fn concurrent_shutdown_callers_wait_for_delayed_cleanup_or_commit() {
        for committing in [false, true] {
            let manager = manager();
            let (id, control) = manager.begin(false, Some("reference".into())).unwrap();
            if committing {
                control.commit().unwrap();
            }
            let (release, waiting) = std::sync::mpsc::channel();
            let done = Arc::new(AtomicBool::new(false));
            let finished = done.clone();
            let worker = manager.clone();
            manager
                .spawn("ohmyloop-delayed-cleanup-fixture", move || {
                    waiting.recv().unwrap();
                    finished.store(true, Ordering::Release);
                    worker.finish(false, &id, &control, Ok(None));
                })
                .unwrap();
            let (completed, closed) = std::sync::mpsc::channel();
            let first_manager = manager.clone();
            let first_complete = completed.clone();
            let first = thread::spawn(move || {
                first_manager.shutdown();
                first_complete.send(()).unwrap();
            });
            let start = Instant::now();
            while !manager.0.state.lock().unwrap().shutting_down {
                assert!(start.elapsed() < std::time::Duration::from_secs(1));
                thread::yield_now();
            }
            let second_manager = manager.clone();
            let second = thread::spawn(move || {
                second_manager.shutdown();
                completed.send(()).unwrap();
            });
            assert!(
                closed
                    .recv_timeout(std::time::Duration::from_millis(30))
                    .is_err(),
                "A concurrent shutdown returned while its worker was still unfinished"
            );
            assert!(!done.load(Ordering::Acquire));
            release.send(()).unwrap();
            closed
                .recv_timeout(std::time::Duration::from_secs(1))
                .unwrap();
            closed
                .recv_timeout(std::time::Duration::from_secs(1))
                .unwrap();
            first.join().unwrap();
            second.join().unwrap();
            assert!(done.load(Ordering::Acquire));
            assert_eq!(
                manager.status().job.phase,
                if committing { "Complete" } else { "Cancelled" }
            );
        }
    }
}
