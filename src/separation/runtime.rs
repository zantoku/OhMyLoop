//! Runtime resolution keeps packaged resources independent from developer tools.
use super::{command, control::Control, managed::Managed};
use serde::Serialize;
use std::{
    path::{Path, PathBuf},
    process::Command,
    sync::Arc,
    time::Duration,
};
#[derive(Clone, Serialize)]
pub struct RuntimeStatus {
    pub ready: bool,
    pub supported: bool,
    pub id: Option<String>,
    pub message: String,
    pub path: Option<String>,
}
#[derive(Clone)]
pub struct Runtime {
    pub python: Option<PathBuf>,
    pub worker: PathBuf,
    pub(super) managed: Option<Arc<Managed>>,
    unavailable: Option<String>,
}
impl Runtime {
    /// Explicit opt-in for the retained CLI prototype and maintainer examples.
    /// The desktop boundary always uses managed(), including development builds.
    pub fn development() -> Self {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        Self {
            python: Some(root.join(if cfg!(windows) {
                ".tools/stems/venv/Scripts/python.exe"
            } else {
                ".tools/stems/venv/bin/python"
            })),
            worker: root.join("resources/separation/worker.py"),
            managed: None,
            unavailable: None,
        }
    }
    pub fn managed(resources: &Path, root: PathBuf) -> Self {
        let worker = resources.join("worker.py");
        match Managed::load(resources, root) {
            Ok(managed) => Self {
                python: Some(managed.python()),
                worker,
                managed: Some(Arc::new(managed)),
                unavailable: None,
            },
            Err(error) => Self {
                python: None,
                worker,
                managed: None,
                unavailable: Some(format!("Separation tools are unavailable: {error:#}")),
            },
        }
    }
    pub fn status(&self) -> RuntimeStatus {
        let ready = self.unavailable.is_none()
            && self.python.as_ref().is_some_and(|p| p.is_file())
            && self.worker.is_file()
            && self.managed.as_ref().is_none_or(|m| m.ready());
        RuntimeStatus {
            ready, supported: self.unavailable.is_none(),
            id: self.managed.as_ref().map(|m| m.definition.id.clone()),
            message: self.unavailable.clone().unwrap_or_else(|| if ready { "Separation tools ready".into() } else { "Set up stem separation to install private tools. Downloads are required; installed separation works offline.".into() }),
            path: self.python.as_ref().map(|p| p.display().to_string()),
        }
    }
    pub fn probe(&self) -> RuntimeStatus {
        let mut status = self.status();
        if status.ready
            && let Err(error) = self.native_check(&Control::default())
        {
            status.ready = false;
            status.message = format!("Separation tools need repair: {error:#}");
        }
        status
    }
    pub(super) fn configure(&self, command: &mut Command) {
        if let Some(managed) = &self.managed {
            super::managed::controlled_environment(command, &managed.ffmpeg(), &managed.root);
        } else {
            command
                .env_remove("PYTHONHOME")
                .env_remove("PYTHONPATH")
                .env("PYTHONNOUSERSITE", "1");
        }
        command
            .env("OMP_NUM_THREADS", "2")
            .env("MKL_NUM_THREADS", "2");
    }
    pub(super) fn native_check(&self, control: &Control) -> anyhow::Result<String> {
        let python = self
            .python
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Separation runtime is unavailable"))?;
        anyhow::ensure!(
            python.is_file() && self.worker.is_file(),
            "Separation runtime is missing"
        );
        let mut cmd = Command::new(python);
        self.configure(&mut cmd);
        cmd.arg("-I").arg(&self.worker).arg("--probe");
        command::run(&mut cmd, control, Duration::from_secs(120))
    }
    #[cfg(test)]
    pub(super) fn fixture(python: PathBuf, worker: PathBuf) -> Self {
        Self {
            python: Some(python),
            worker,
            managed: None,
            unavailable: None,
        }
    }
}
