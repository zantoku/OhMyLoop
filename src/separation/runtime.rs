//! Explicit development runtime resolution; release packaging belongs to issue #7.
use serde::Serialize;
use std::{
    path::PathBuf,
    process::{Command, Stdio},
    thread,
    time::{Duration, Instant},
};
#[derive(Clone, Serialize)]
pub struct RuntimeStatus {
    pub ready: bool,
    pub message: String,
    pub path: Option<String>,
}
#[derive(Clone)]
pub struct Runtime {
    pub python: Option<PathBuf>,
    pub worker: PathBuf,
}
impl Runtime {
    pub fn development() -> Self {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let python = if cfg!(debug_assertions) {
            Some(root.join(if cfg!(windows) {
                ".tools/stems/venv/Scripts/python.exe"
            } else {
                ".tools/stems/venv/bin/python"
            }))
        } else {
            None
        };
        Self {
            python,
            worker: root.join("resources/separation/worker.py"),
        }
    }
    pub fn probe(&self) -> RuntimeStatus {
        let setup = "Run scripts/stem_prototype/setup.sh in this workspace, and install FFmpeg on PATH. Production runtime packaging is issue #7.";
        let Some(python) = &self.python else {
            return RuntimeStatus{ready:false,message:"An installed inference runtime is not packaged yet (issue #7). Use the development build with its prepared workspace runtime.".into(),path:None};
        };
        let result = (|| -> anyhow::Result<()> {
            anyhow::ensure!(
                python.is_file() && self.worker.is_file(),
                "Development runtime is missing"
            );
            let mut child = Command::new(python)
                .arg("-I")
                .arg(&self.worker)
                .arg("--probe")
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()?;
            let began = Instant::now();
            loop {
                if let Some(exit) = child.try_wait()? {
                    anyhow::ensure!(
                        exit.success(),
                        "Runtime versions or FFmpeg prerequisite differ from the pinned profile"
                    );
                    break;
                }
                if began.elapsed() > Duration::from_secs(10) {
                    let _ = child.kill();
                    let _ = child.wait();
                    anyhow::bail!("Runtime readiness check timed out");
                }
                thread::sleep(Duration::from_millis(25));
            }
            Ok(())
        })();
        RuntimeStatus {
            ready: result.is_ok(),
            message: match result {
                Ok(()) => "Pinned Python 3.12.13 / Roformer CPU runtime ready".into(),
                Err(e) => format!("{e:#}. {setup}"),
            },
            path: Some(python.display().to_string()),
        }
    }
}
