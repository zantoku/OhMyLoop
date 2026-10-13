// SPDX-License-Identifier: GPL-3.0-or-later
//! Process-resident memory sampling; excludes external OS filesystem cache.
use anyhow::{Context, Result, ensure};
use std::{
    process::Command,
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
    thread::{self, JoinHandle},
    time::Duration,
};
pub fn resident_bytes() -> Result<u64> {
    let output = Command::new("ps")
        .args(["-o", "rss=", "-p", &std::process::id().to_string()])
        .output()
        .context("Run ps for process RSS")?;
    ensure!(output.status.success(), "ps RSS measurement failed");
    Ok(String::from_utf8(output.stdout)?.trim().parse::<u64>()? * 1024)
}
pub struct MemorySampler {
    pub baseline: u64,
    pub peak: Arc<AtomicU64>,
    stop: Arc<AtomicBool>,
    worker: Option<JoinHandle<()>>,
}
impl MemorySampler {
    pub fn start() -> Result<Self> {
        let baseline = resident_bytes()?;
        let peak = Arc::new(AtomicU64::new(baseline));
        let stop = Arc::new(AtomicBool::new(false));
        let (p, s) = (peak.clone(), stop.clone());
        let worker = thread::spawn(move || {
            while !s.load(Ordering::Relaxed) {
                if let Ok(bytes) = resident_bytes() {
                    p.fetch_max(bytes, Ordering::Relaxed);
                }
                thread::sleep(Duration::from_millis(10));
            }
        });
        Ok(Self {
            baseline,
            peak,
            stop,
            worker: Some(worker),
        })
    }
    pub fn peak(&self) -> u64 {
        self.peak.load(Ordering::Relaxed)
    }
}
impl Drop for MemorySampler {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}
