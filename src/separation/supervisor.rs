//! Direct child supervision with bounded pipes and owned process-tree cleanup.
use super::{control::Control, protocol, runtime::Runtime};
use anyhow::{Context, Result};
use std::{
    io::BufReader,
    path::Path,
    process::{Child, Command, Stdio},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    thread,
    time::Duration,
};
fn kill_tree(child: &mut Child) {
    #[cfg(unix)]
    {
        // SAFETY: child is the leader of the private process group created below.
        unsafe {
            libc::kill(-(child.id() as i32), libc::SIGKILL);
        }
    }

    let _ = child.kill();
}
pub fn run(
    runtime: &Runtime,
    request: &Path,
    job: &str,
    control: &Control,
    publish: impl FnMut(protocol::Update) + Send + 'static,
) -> Result<String> {
    let python = runtime
        .python
        .as_ref()
        .context("Development runtime unavailable")?;
    let mut command = Command::new(python);
    runtime.configure(&mut command);
    command
        .arg("-I")
        .arg(&runtime.worker)
        .arg(request)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .env("OMP_NUM_THREADS", "2")
        .env("MKL_NUM_THREADS", "2");
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        command.process_group(0);
    }
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000);
    }
    control.check()?;
    let mut child = command
        .spawn()
        .context("Could not start the separation process")?;
    #[cfg(windows)]
    let process_tree = match super::process_tree::ProcessTree::assign(&child) {
        Ok(tree) => tree,
        Err(error) => {
            kill_tree(&mut child);
            let _ = child.wait();
            return Err(error);
        }
    };
    let stdout = child
        .stdout
        .take()
        .context("Missing worker progress pipe")?;
    let stderr = child
        .stderr
        .take()
        .context("Missing worker diagnostic pipe")?;
    let failed = Arc::new(AtomicBool::new(false));
    let protocol_error = Arc::new(Mutex::new(None::<String>));
    let failure = failed.clone();
    let error = protocol_error.clone();
    let job = job.to_owned();
    let reader = thread::spawn(move || {
        if let Err(e) = protocol::read_updates(BufReader::new(stdout), &job, publish) {
            *error.lock().unwrap() = Some(format!("{e:#}"));
            failure.store(true, Ordering::Release);
        }
    });
    let diagnostic = thread::spawn(move || protocol::tail(stderr));
    let status = (|| {
        loop {
            if control.cancelled() || failed.load(Ordering::Acquire) {
                kill_tree(&mut child);
            }
            if let Some(exit) = child.try_wait()? {
                return Ok::<_, anyhow::Error>(exit);
            }
            thread::sleep(Duration::from_millis(25));
        }
    })();
    // Reap and terminate any inherited pipe-owning descendants even after clean parent exit.
    kill_tree(&mut child);
    let _ = child.wait();
    #[cfg(windows)]
    drop(process_tree);
    let _ = reader.join();
    let bytes = diagnostic
        .join()
        .unwrap_or_else(|_| Ok(Vec::new()))
        .unwrap_or_default();
    let details = String::from_utf8_lossy(&bytes).into_owned();
    control.check()?;
    if let Some(e) = protocol_error.lock().unwrap().take() {
        return Err(anyhow::anyhow!("{e}\n{details}")
            .context("The separation worker sent invalid progress"));
    }
    if !status?.success() {
        return Err(
            anyhow::anyhow!("{details}").context("The separation process stopped unexpectedly")
        );
    }
    Ok(details)
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use std::{fs, os::unix::fs::PermissionsExt, time::Instant};
    fn runtime(body: &str) -> (Runtime, std::path::PathBuf) {
        let root = std::env::temp_dir().join(uuid::Uuid::new_v4().to_string());
        fs::create_dir(&root).unwrap();
        let executable = root.join("worker");
        fs::write(&executable, format!("#!/bin/sh\n{body}\n")).unwrap();
        fs::set_permissions(&executable, fs::Permissions::from_mode(0o700)).unwrap();
        (Runtime::fixture(executable, root.join("unused")), root)
    }
    #[test]
    fn crashes_protocol_failures_and_forked_pipe_holders_are_reaped() {
        let update = r#"echo '{"version":1,"job_id":"job","stage":"worker.load","completed":null,"total":null,"unit":null}'"#;
        let (r, root) = runtime(&format!("{update}\nsleep 60 &\nexit 0"));
        let start = Instant::now();
        assert!(run(&r, Path::new("unused"), "job", &Control::default(), |_| {}).is_ok());
        assert!(start.elapsed() < Duration::from_secs(2));
        fs::remove_dir_all(root).unwrap();
        let (r, root) = runtime("echo broken\nsleep 60");
        let start = Instant::now();
        let error = run(&r, Path::new("unused"), "job", &Control::default(), |_| {}).unwrap_err();
        assert_eq!(
            error.to_string(),
            "The separation worker sent invalid progress"
        );
        assert!(start.elapsed() < Duration::from_secs(2));
        fs::remove_dir_all(root).unwrap();
        let (r, root) = runtime("echo diagnostic >&2\nexit 7");
        let error = run(&r, Path::new("unused"), "job", &Control::default(), |_| {}).unwrap_err();
        assert_eq!(
            error.to_string(),
            "The separation process stopped unexpectedly"
        );
        assert!(format!("{error:#}").contains("diagnostic"));
        fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn hung_worker_and_descendants_cancel_responsively() {
        let (r, root) = runtime("sleep 60 &\nwait");
        let control = Control::default();
        let cancel = control.clone();
        let start = Instant::now();
        let job = thread::spawn(move || run(&r, Path::new("unused"), "job", &control, |_| {}));
        thread::sleep(Duration::from_millis(100));
        cancel.cancel();
        assert!(job.join().unwrap().is_err());
        assert!(start.elapsed() < Duration::from_secs(2));
        fs::remove_dir_all(root).unwrap();
    }
}
