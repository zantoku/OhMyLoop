//! Bounded, cancellable execution for setup and native readiness checks.
use super::{control::Control, protocol};
use anyhow::{Context, Result, ensure};
use std::{
    process::{Child, Command, Stdio},
    thread,
    time::{Duration, Instant},
};

pub(super) fn own(command: &mut Command) {
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
}
pub(super) fn stop(child: &mut Child) {
    #[cfg(unix)]
    {
        // SAFETY: own() creates this child's private process group before spawn.
        unsafe {
            libc::kill(-(child.id() as i32), libc::SIGKILL);
        }
    }
    let _ = child.kill();
}
pub(super) fn run(command: &mut Command, control: &Control, timeout: Duration) -> Result<String> {
    control.check()?;
    own(command);
    let mut child = command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .context("Could not start separation tools")?;
    #[cfg(windows)]
    let tree = match super::process_tree::ProcessTree::assign(&child) {
        Ok(tree) => tree,
        Err(error) => {
            stop(&mut child);
            let _ = child.wait();
            return Err(error);
        }
    };
    let stdout = child.stdout.take().context("Missing tools output pipe")?;
    let stderr = child
        .stderr
        .take()
        .context("Missing tools diagnostic pipe")?;
    let output = thread::spawn(move || protocol::tail(stdout));
    let diagnostic = thread::spawn(move || protocol::tail(stderr));
    let began = Instant::now();
    let status = loop {
        if control.cancelled() || began.elapsed() >= timeout {
            stop(&mut child);
        }
        match child.try_wait() {
            Ok(Some(exit)) => break Ok(exit),
            Ok(None) => thread::sleep(Duration::from_millis(25)),
            Err(error) => break Err(error),
        }
    };
    // Descendants must not retain our pipes, even if their parent exited first.
    stop(&mut child);
    let _ = child.wait();
    #[cfg(windows)]
    drop(tree);
    let mut bytes = output
        .join()
        .unwrap_or_else(|_| Ok(Vec::new()))
        .unwrap_or_default();
    bytes.extend(
        diagnostic
            .join()
            .unwrap_or_else(|_| Ok(Vec::new()))
            .unwrap_or_default(),
    );
    let start = bytes.len().saturating_sub(protocol::DIAGNOSTIC_BYTES);
    let details = String::from_utf8_lossy(&bytes[start..]).into_owned();
    control.check()?;
    ensure!(
        began.elapsed() < timeout,
        "Separation tools timed out\n{details}"
    );
    ensure!(status?.success(), "Separation tools failed\n{details}");
    Ok(details)
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    #[test]
    fn checks_timeout_and_reaps_pipe_holding_descendants() {
        let began = Instant::now();
        run(
            Command::new("/bin/sh").args(["-c", "sleep 60 & exit 0"]),
            &Control::default(),
            Duration::from_secs(2),
        )
        .unwrap();
        assert!(began.elapsed() < Duration::from_secs(2));
        let error = run(
            Command::new("/bin/sh").args(["-c", "sleep 60 & wait"]),
            &Control::default(),
            Duration::from_millis(100),
        )
        .unwrap_err();
        assert!(error.to_string().contains("timed out"));
    }
    #[test]
    fn cancels_before_spawning_and_bounds_noisy_failure() {
        let control = Control::default();
        control.cancel();
        assert!(
            run(
                &mut Command::new("/no/such/tool"),
                &control,
                Duration::from_secs(1)
            )
            .unwrap_err()
            .to_string()
            .contains("Cancelled")
        );
        let error = run(
            Command::new("/bin/sh").args([
                "-c",
                "i=0; while [ $i -lt 2000 ]; do echo diagnostic-output; i=$((i+1)); done; exit 1",
            ]),
            &Control::default(),
            Duration::from_secs(2),
        )
        .unwrap_err();
        assert!(error.to_string().len() < protocol::DIAGNOSTIC_BYTES + 100);
    }
}
