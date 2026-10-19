//! Aggregate setup reuses independently completed runtime and model components.
use super::*;
impl SeparationManager {
    pub fn install(&self) -> Result<Status> {
        self.setup(false, false)
    }
    pub fn reinstall_runtime(&self) -> Result<Status> {
        self.setup(true, false)
    }
    pub fn reinstall_model(&self) -> Result<Status> {
        self.setup(false, true)
    }
    fn setup(&self, reinstall_runtime: bool, reinstall_model: bool) -> Result<Status> {
        ensure!(
            self.0.runtime.status().supported,
            "{}",
            self.0.runtime.status().message
        );
        let (id, control) = self.begin(true, None)?;
        let manager = self.clone();
        let failid = id.clone();
        let failcontrol = control.clone();
        if let Err(e) = self.spawn("ohmyloop-stem-setup", move || {
            let result = manager
                .setup_components(&id, &control, reinstall_runtime, reinstall_model)
                .map(|()| None);
            manager.0.state.lock().unwrap().runtime = manager.0.runtime.status();
            manager.finish(true, &id, &control, result);
        }) {
            self.finish(true, &failid, &failcontrol, Err(e));
        }
        Ok(self.status())
    }
    fn setup_components(
        &self,
        id: &str,
        control: &Control,
        reinstall_runtime: bool,
        reinstall_model: bool,
    ) -> Result<()> {
        if let Some(managed) = &self.0.runtime.managed {
            if reinstall_runtime {
                managed.prepare_reinstall()?;
            }
            managed.install(&self.0.runtime, control, |stage| {
                self.setup_progress(id, stage, 0, None)
            })?;
        } else {
            self.0.runtime.native_check(control)?;
        }
        self.0.state.lock().unwrap().runtime = self.0.runtime.status();
        control.check()?;
        if reinstall_model {
            package::remove(&self.0.models)?;
        }
        package::install(&self.0.models, control, |n, t| {
            self.setup_progress(id, "model.download", n, Some(t))
        })?;
        control.check()?;
        self.setup_progress(id, "complete", 1, Some(1));
        Ok(())
    }
    fn setup_progress(&self, id: &str, stage: &str, completed: u64, total: Option<u64>) {
        let index = managed::STAGES
            .iter()
            .position(|(key, _)| *key == stage)
            .unwrap_or(managed::STAGES.len());
        let steps = managed::STAGES
            .iter()
            .enumerate()
            .map(|(i, (key, label))| crate::stems::library::WorkflowStep {
                id: (*key).into(),
                label: (*label).into(),
                state: if i < index {
                    "complete"
                } else if i == index {
                    "active"
                } else {
                    "pending"
                },
                completed: if i == index { completed } else { 0 },
                total: if i == index { total } else { None },
            })
            .collect();
        let finished = index == managed::STAGES.len();
        let phase = managed::STAGES
            .get(index)
            .map(|(_, label)| *label)
            .unwrap_or("Stem separation ready");
        self.progress(
            true,
            id,
            phase,
            Some(Progress {
                stage: stage.into(),
                completed,
                total,
                workflow: Some(crate::stems::library::WorkflowProgress {
                    steps,
                    completed: if finished { 1. } else { 0. },
                    total: usize::from(finished),
                }),
            }),
        );
    }
    pub fn remove_runtime(&self) -> Result<Status> {
        let mut state = self.0.state.lock().unwrap();
        ensure!(
            !state.install.running && !state.job.running,
            "Separation tools are in use"
        );
        self.0
            .runtime
            .managed
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Managed tools are unavailable"))?
            .remove()?;
        state.runtime = self.0.runtime.status();
        drop(state);
        Ok(self.status())
    }
}
