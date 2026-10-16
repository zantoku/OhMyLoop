//! One owned generation: admission, worker execution, validation and publication.
use super::*;
use crate::{
    desktop::{Desktop, stems::Operation},
    heavy_work::Permit,
};
use anyhow::Context;
use serde_json::json;
struct Job {
    id: String,
    control: Control,
    operation: Operation,
    owned: PathBuf,
    set: String,
    reference: PathBuf,
    reference_state: prepare::Freshness,
    output_rate: u32,
}
impl SeparationManager {
    pub fn start(&self, desktop: Desktop, token: String) -> Result<Status> {
        {
            let state = self.0.state.lock().unwrap();
            ensure!(state.runtime.ready, "{}", state.runtime.message);
            ensure!(
                !state.install.running && !state.job.running,
                "Model or separation work is already running"
            );
        }
        ensure!(package::present(&self.0.models), "Download the model first");
        let permit = self.0.gate.acquire("Stem separation")?;
        let context = desktop.separation_context(token.clone())?;
        let reference = context.reference()?;
        let output_rate = crate::playback::Player::output_rate().unwrap_or(context.rate());
        let set = uuid::Uuid::new_v4().to_string();
        let owned = context.owned(&set)?;
        let parent = owned.parent().context("Missing stem storage")?;
        std::fs::create_dir_all(parent)?;
        disk::admit(
            parent,
            disk::estimate(context.frames(), context.rate(), output_rate)?,
        )?;
        let reference_state = prepare::freshness(&reference)?;
        let operation = desktop.begin_separation(token.clone())?;
        let (id, control) = match self.begin(false, Some(token)) {
            Ok(value) => value,
            Err(error) => {
                let _ = desktop.finish_separation();
                return Err(error);
            }
        };
        let job = Job {
            id: id.clone(),
            control: control.clone(),
            operation,
            owned,
            set,
            reference,
            reference_state,
            output_rate,
        };
        let manager = self.clone();
        let failure_desktop = desktop.clone();
        if let Err(error) = self.spawn("ohmyloop-separation", move || {
            manager.work_job(desktop, job, permit)
        }) {
            let _ = failure_desktop.finish_separation();
            self.finish(false, &id, &control, Err(error));
        }
        Ok(self.status())
    }
    fn work_job(&self, desktop: Desktop, job: Job, permit: Permit) {
        let result = self.execute_job(&desktop, &job);
        let result = if result.is_err() {
            match job.operation.cleanup(&job.set) {
                Ok(()) => result,
                Err(cleanup) => result.map_err(|error| {
                    error.context(format!(
                        "Uncommitted output cleanup failed at {}: {cleanup:#}",
                        job.owned.display()
                    ))
                }),
            }
        } else {
            result
        };
        let _ = desktop.finish_separation();
        drop(permit);
        self.finish(false, &job.id, &job.control, result);
    }
    fn execute_job(&self, desktop: &Desktop, job: &Job) -> Result<Option<Value>> {
        let Job {
            id,
            control,
            operation,
            owned,
            reference,
            reference_state,
            output_rate,
            ..
        } = job;
        self.measured(id, "model.verify", "Verifying model", 0, None);
        control.check()?;
        ensure!(!owned.exists(), "Generated destination already exists");
        std::fs::create_dir(owned)?;
        let package_before = package::snapshot(&self.0.models)?;
        package::verify(&self.0.models, control, |completed, total| {
            self.measured(
                id,
                "model.verify",
                "Verifying model",
                completed,
                Some(total),
            )
        })?;
        ensure!(
            package::snapshot(&self.0.models)? == package_before,
            "Model changed during verification; reinstall it"
        );
        ensure!(
            prepare::freshness(reference)? == *reference_state,
            "Reference changed before processing"
        );
        let (preparation, checked) = prepare::prepare(
            reference,
            owned,
            operation.rate(),
            operation.frames(),
            control,
            |stage, completed, total| {
                self.measured(id, stage, "Preparing recording", completed, total)
            },
        )?;
        ensure!(
            preparation["source_sha256"] == operation.digest(),
            "Reference digest changed; reopen the song"
        );
        let request = self.worker_request(job, &preparation);
        let bytes = serde_json::to_vec(&request)?;
        ensure!(
            bytes.len() as u64 <= protocol::MAX_DOCUMENT,
            "Worker request exceeds limit"
        );
        std::fs::write(owned.join("request.json"), bytes)?;
        ensure!(
            package::snapshot(&self.0.models)? == package_before,
            "Model changed before inference; reinstall it"
        );
        self.measured(id, "worker.start", "Starting separation worker", 0, None);
        let observer = self.clone();
        let observer_id = id.clone();
        let diagnostics = supervisor::run(
            &self.0.runtime,
            &owned.join("request.json"),
            id,
            control,
            move |progress| {
                let phase = match progress.stage.as_str() {
                    "worker.start" | "worker.input" => "Preparing separation worker",
                    "worker.load" => "Loading model",
                    "worker.finish" => "Finishing separation worker",
                    "worker.infer" => "Separating instruments",
                    _ => "Writing separated tracks",
                };
                observer.measured(
                    &observer_id,
                    &progress.stage,
                    phase,
                    progress.completed.unwrap_or(0),
                    progress.total,
                );
            },
        )?;
        self.0.state.lock().unwrap().job.details = bounded_details(&diagnostics);
        self.measured(id, "result.validate", "Checking separated outputs", 0, None);
        control.check()?;
        ensure!(
            package::snapshot(&self.0.models)? == package_before,
            "Model changed during inference; discard results"
        );
        let result = protocol::read_result(&owned.join("result.json"), id, &preparation)?;
        let manifest = protocol::manifest(&preparation, &result)?;
        std::fs::write(owned.join("manifest.json"), serde_json::to_vec(&manifest)?)?;
        let (record, prepared) = crate::stems::library::attach_generated(
            reference,
            owned,
            *output_rate,
            control,
            |progress| self.preparation(id, progress),
        )?;
        ensure!(
            prepare::freshness(reference)? == checked && checked == *reference_state,
            "Reference changed before saving"
        );
        control.check()?;
        for name in [
            "prepared.wav",
            "request.json",
            "result.json",
            "manifest.json",
        ] {
            std::fs::remove_file(owned.join(name))?;
        }
        // Cancellation and committing share one mutex; there is no cancellable save.
        control.commit()?;
        self.measured(id, "attachment.publish", "Finishing save", 0, Some(1));
        self.0.state.lock().unwrap().job.cancellable = false;
        let status = desktop.publish_separation(operation.clone(), record, prepared)?;
        self.measured(id, "attachment.publish", "Finishing save", 1, Some(1));
        Ok(Some(serde_json::to_value(status)?))
    }
    pub(super) fn measured(
        &self,
        id: &str,
        stage: &str,
        phase: &str,
        completed: u64,
        total: Option<u64>,
    ) {
        self.progress(
            false,
            id,
            phase,
            Some(Progress {
                stage: stage.into(),
                completed,
                total,
                workflow: None,
            }),
        );
    }
    fn worker_request(&self, job: &Job, preparation: &Value) -> Value {
        json!({"version":1,"job_id":job.id,"package_id":package::ID,"profile_id":package::PROFILE,"model_dir":self.0.models,"prepared_path":job.owned.join("prepared.wav"),"preparation":preparation,"settings":protocol::settings(),"output_dir":job.owned,"outputs":crate::stems::Role::ALL.into_iter().map(|role|(role.name().to_owned(),json!(format!("{}.wav",role.name())))).collect::<serde_json::Map<_,_>>()})
    }
}
