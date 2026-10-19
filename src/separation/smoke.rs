//! Packaged-binary maintainer checks. Uses production ownership and publication.
use super::{SeparationManager, Status};
use crate::{desktop::Desktop, heavy_work::HeavyWork};
use anyhow::{Context, Result, ensure};
use std::{
    fs,
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
    thread,
    time::{Duration, Instant},
};

/// Called only by the installed desktop executable's explicit QA command.
pub fn run(args: &[String], resources: &Path) -> Result<()> {
    ensure!(
        args.len() >= 2,
        "Usage: --stem-runtime-smoke setup|cancel-setup|check|run-short|run-normal|cancel-run|remove-tools|remove-model|portable ISOLATED_ROOT [SOURCE]"
    );
    let mode = args[0].as_str();
    let root = std::path::absolute(&args[1])?;
    fs::create_dir_all(&root)?;
    let root = root.canonicalize()?;
    let manager = SeparationManager::managed(
        root.join("models"),
        root.join("tools"),
        resources,
        HeavyWork::default(),
    );
    println!("packaged_resources {}", resources.display());
    print_status("initial", &manager.status())?;
    match mode {
        "setup" | "cancel-setup" | "shutdown-setup" => {
            let before = Instant::now();
            let disk = DiskMonitor::start(root.clone());
            manager.install()?;
            wait(
                &manager,
                true,
                if mode != "setup" {
                    Some("runtime.dependencies")
                } else {
                    None
                },
                mode == "shutdown-setup",
            )?;
            let status = manager.status();
            if mode == "setup" {
                ensure!(
                    status.runtime.ready && status.model_ready,
                    "Setup did not make tools and model Ready"
                );
            } else {
                ensure!(
                    status.install.phase == "Cancelled",
                    "Setup was not cancelled"
                );
            }
            println!(
                "setup_metrics {{\"elapsed_seconds\":{},\"app_data_bytes\":{},\"sampled_peak_disk_bytes\":{}}}",
                before.elapsed().as_secs_f64(),
                tree_bytes(&root)?,
                disk.peak()
            );
        }
        "check" => {
            ensure!(
                manager.status().runtime.ready && manager.status().model_ready,
                "Setup is required"
            );
            manager
                .0
                .runtime
                .native_check(&super::control::Control::default())?;
            if let Some(managed) = &manager.0.runtime.managed {
                // uv leaves small bookkeeping files after cache clean; no wheels/extracted copies.
                ensure!(
                    tree_bytes(&managed.cache())? < 1024 * 1024,
                    "Disposable runtime downloads remain after setup"
                );
            }
        }
        "remove-tools" => {
            manager.remove_runtime()?;
            ensure!(
                !manager.status().runtime.ready,
                "Tools remain Ready after removal"
            );
        }
        "remove-model" => {
            manager.remove_model()?;
            ensure!(
                !manager.status().model_ready,
                "Model remains Ready after removal"
            );
        }
        "run-short" | "run-normal" | "cancel-run" | "shutdown-run" | "portable"
        | "verify-completed" => {
            let source = if let Some(source) = args.get(2) {
                PathBuf::from(source)
            } else {
                let seconds = if mode == "run-short" { 1 } else { 20 };
                let source = root.join(format!("generated-{seconds}s.wav"));
                if !source.exists() {
                    generate(&source, seconds)?;
                }
                source
            };
            run_song(&manager, &root, &source, mode)?;
        }
        _ => anyhow::bail!("Unknown packaged smoke mode"),
    }
    print_status("terminal", &manager.status())?;
    manager.shutdown();
    Ok(())
}
fn print_status(label: &str, status: &Status) -> Result<()> {
    println!("{label} {}", serde_json::to_string(status)?);
    Ok(())
}
fn wait(
    manager: &SeparationManager,
    install: bool,
    cancel: Option<&str>,
    shutdown: bool,
) -> Result<()> {
    let mut previous = String::new();
    let mut cancelled = false;
    loop {
        let status = manager.status();
        let attempt = if install {
            &status.install
        } else {
            &status.job
        };
        let stage = attempt
            .progress
            .as_ref()
            .map(|p| p.stage.as_str())
            .unwrap_or("");
        let key = format!("{}:{stage}", attempt.phase);
        if previous != key {
            println!("attempt {}", serde_json::to_string(attempt)?);
            previous = key;
        }
        if attempt.running && !cancelled && cancel.is_some_and(|v| stage.starts_with(v)) {
            if shutdown {
                manager.shutdown();
            } else {
                manager.cancel(&attempt.id);
            }
            cancelled = true;
        }
        if !attempt.running {
            ensure!(
                if cancelled {
                    attempt.phase == "Cancelled"
                } else {
                    attempt.phase == "Complete"
                },
                "{}\n{}",
                attempt.error.as_deref().unwrap_or(&attempt.phase),
                attempt.details
            );
            if cancel.is_some() {
                ensure!(cancelled, "Requested cancellation stage was never reached");
            }
            return Ok(());
        }
        thread::sleep(Duration::from_millis(25));
    }
}
fn run_song(manager: &SeparationManager, root: &Path, source: &Path, mode: &str) -> Result<()> {
    let desktop = Desktop::start(root.join("practice"))?;
    let pointer = root.join("library-root.txt");
    if pointer.exists() {
        desktop.select_library(fs::read_to_string(&pointer)?, false, false)?;
    } else {
        let parent = root.join("music");
        fs::create_dir_all(&parent)?;
        let snapshot = desktop.select_library(parent.display().to_string(), true, false)?;
        fs::write(
            &pointer,
            snapshot
                .library_status
                .selected
                .context("Missing library")?
                .root,
        )?;
    }
    let candidates = desktop.import_candidates(source.display().to_string())?;
    let snapshot = if let Some(candidate) = candidates.first() {
        desktop.open_library(candidate.song_key.clone())?
    } else {
        desktop.import_managed(source.display().to_string(), |_, _| {})?
    };
    let song = snapshot.song.context("Missing generated song")?;
    let mut authored = snapshot.document.context("Missing authored document")?;
    if authored.notes.is_empty() {
        authored.notes = "Managed runtime QA — preserve these notes, map and sections.".into();
        let frames = (song.duration * f64::from(song.rate)).round() as u64;
        authored.measure_map = Some(
            crate::timing::MeasureMap::empty(song.rate, frames)
                .edit(crate::timing::Edit::Build {
                    start: 0,
                    end: frames,
                    count: 1,
                    meter: crate::timing::Meter::common(),
                })?
                .map,
        );
        authored.sections.push(crate::project::Section {
            id: "managed-runtime-qa".into(),
            name: "Authored phrase".into(),
            start: 0.,
            end: song.duration * 0.75,
        });
        authored.controls.speed = 0.8;
        authored.controls.semitones = 2.;
        desktop.update(authored.clone())?;
    }
    let before = desktop.stems_status(song.reference_token.clone())?;
    if mode == "verify-completed" {
        ensure!(
            before.available && before.set_id.is_some() && !before.tracks.is_empty(),
            "Completed stems need unavailable inference tools"
        );
        let mut tracks = before.tracks;
        tracks[0].gain = 0.5;
        let selected =
            desktop.set_stem_mix(song.reference_token, before.set_id.unwrap(), tracks, false)?;
        ensure!(selected.available, "Completed audio could not be selected");
        println!(
            "completed_audio_without_tools {}",
            serde_json::to_string(&selected)?
        );
        desktop.shutdown();
        return Ok(());
    }
    if mode == "portable" {
        ensure!(
            before.available && before.set_id.is_some(),
            "Completed stems unavailable before library move"
        );
        desktop.shutdown();
        let old = PathBuf::from(fs::read_to_string(&pointer)?);
        let inventory = audio_inventory(&old)?;
        let moved = root.join("Moved music library ü");
        fs::rename(&old, &moved)?;
        for path in [&moved, &root.join("Copied music library ü")] {
            if path != &moved {
                copy_tree(&moved, path)?;
            }
            ensure!(
                audio_inventory(path)? == inventory,
                "Library move/copy changed recording or stem bytes"
            );
            let reopened = Desktop::start(root.join("practice"))?;
            reopened.select_library(path.display().to_string(), false, true)?;
            let next = reopened.open_library(song.key.clone())?;
            let next_song = next.song.context("Missing moved/copied song")?;
            ensure!(
                next_song.rate == song.rate
                    && next_song.duration == song.duration
                    && next_song.asset_id == song.asset_id
                    && next_song.timeline_id == song.timeline_id,
                "Library move/copy changed recording identity or geometry"
            );
            let after = reopened.stems_status(next_song.reference_token)?;
            ensure!(
                after.available && after.set_id == before.set_id,
                "Library move/copy lost completed stems"
            );
            same_authored(
                &authored,
                &next
                    .document
                    .context("Missing moved/copied authored document")?,
            )?;
            reopened.shutdown();
        }
        fs::write(pointer, moved.display().to_string())?;
        return Ok(());
    }
    let began = Instant::now();
    manager.start(desktop.clone(), song.reference_token.clone())?;
    let result = wait(
        manager,
        false,
        if mode == "cancel-run" || mode == "shutdown-run" {
            Some("worker.infer")
        } else {
            None
        },
        mode == "shutdown-run",
    );
    if result.is_ok() {
        let after = desktop.stems_status(song.reference_token.clone())?;
        if mode == "cancel-run" || mode == "shutdown-run" {
            ensure!(
                after.set_id == before.set_id,
                "Cancellation changed the attached stem set"
            );
        } else {
            ensure!(
                after.available && after.set_id.is_some() && !after.tracks.is_empty(),
                "No audible validated stems were produced"
            );
        }
        println!(
            "separation_metrics {{\"elapsed_seconds\":{},\"stages\":{}}}",
            began.elapsed().as_secs_f64(),
            serde_json::to_string(&manager.timings())?
        );
        let library =
            crate::managed::ManagedLibrary::open(Path::new(&fs::read_to_string(&pointer)?))?;
        let saved = library.record(&song.key)?.record;
        ensure!(
            saved.notes == authored.notes && saved.timeline.measure_map == authored.measure_map,
            "Separation changed authored notes or map"
        );
        println!("completed_audio {}", serde_json::to_string(&after)?);
    }
    manager.shutdown();
    desktop.shutdown();
    result
}
fn generate(path: &Path, seconds: u32) -> Result<()> {
    let mut writer = hound::WavWriter::create(
        path,
        hound::WavSpec {
            channels: 2,
            sample_rate: 44100,
            bits_per_sample: 32,
            sample_format: hound::SampleFormat::Float,
        },
    )?;
    for frame in 0..seconds * 44100 {
        let t = frame as f32 / 44100.;
        let hit = (1. - (t * 2.).fract()).powi(12);
        let bass = (std::f32::consts::TAU * 110. * t).sin() * 0.16;
        let chord = [220., 277.18, 329.63, 440.]
            .into_iter()
            .map(|f| (std::f32::consts::TAU * f * t).sin() * 0.035)
            .sum::<f32>();
        let percussion =
            ((frame.wrapping_mul(1664525).wrapping_add(1013904223) >> 8) as f32 / 16777216. - 0.5)
                * hit
                * 0.25;
        let sample = bass + chord + percussion;
        writer.write_sample(sample)?;
        writer.write_sample(sample * 0.97)?;
    }
    writer.finalize()?;
    Ok(())
}
pub fn tree_bytes(root: &Path) -> Result<u64> {
    if !root.exists() {
        return Ok(0);
    }
    let meta = fs::symlink_metadata(root)?;
    if meta.file_type().is_symlink() {
        return Ok(0);
    }
    if meta.is_file() {
        return Ok(meta.len());
    }
    let mut total = 0u64;
    for entry in fs::read_dir(root)? {
        total = total
            .checked_add(tree_bytes(&entry?.path())?)
            .context("Disk measurement overflow")?;
    }
    Ok(total)
}
struct DiskMonitor {
    stop: Arc<AtomicBool>,
    peak: Arc<AtomicU64>,
    worker: Option<thread::JoinHandle<()>>,
}
impl DiskMonitor {
    fn start(root: PathBuf) -> Self {
        let stop = Arc::new(AtomicBool::new(false));
        let peak = Arc::new(AtomicU64::new(0));
        let (done, maximum) = (stop.clone(), peak.clone());
        let worker = thread::spawn(move || {
            while !done.load(Ordering::Acquire) {
                if let Ok(bytes) = tree_bytes(&root) {
                    maximum.fetch_max(bytes, Ordering::Relaxed);
                }
                thread::sleep(Duration::from_millis(200));
            }
        });
        Self {
            stop,
            peak,
            worker: Some(worker),
        }
    }
    fn peak(&self) -> u64 {
        self.peak.load(Ordering::Relaxed)
    }
}
impl Drop for DiskMonitor {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

fn same_authored(
    before: &crate::project::Document,
    after: &crate::project::Document,
) -> Result<()> {
    ensure!(
        before.notes == after.notes
            && before.measure_map == after.measure_map
            && before.sections == after.sections
            && before.controls == after.controls,
        "Library move/copy changed authored notes, map, sections or controls"
    );
    Ok(())
}
fn copy_tree(from: &Path, to: &Path) -> Result<()> {
    let meta = fs::symlink_metadata(from)?;
    ensure!(
        !meta.file_type().is_symlink(),
        "QA library contains an unexpected symlink"
    );
    if meta.is_dir() {
        fs::create_dir(to)?;
        for entry in fs::read_dir(from)? {
            let entry = entry?;
            copy_tree(&entry.path(), &to.join(entry.file_name()))?;
        }
    } else {
        fs::copy(from, to)?;
    }
    Ok(())
}
fn audio_inventory(root: &Path) -> Result<Vec<(PathBuf, String)>> {
    fn collect(root: &Path, path: &Path, values: &mut Vec<(PathBuf, String)>) -> Result<()> {
        let meta = fs::symlink_metadata(path)?;
        ensure!(
            !meta.file_type().is_symlink(),
            "QA library contains an unexpected symlink"
        );
        if meta.is_dir() {
            for entry in fs::read_dir(path)? {
                collect(root, &entry?.path(), values)?;
            }
        } else if path.extension().is_none_or(|e| e != "json") {
            values.push((
                path.strip_prefix(root)?.to_path_buf(),
                super::package::digest(path, &super::control::Control::default(), |_, _| {})?,
            ));
        }
        Ok(())
    }
    let mut values = Vec::new();
    collect(root, root, &mut values)?;
    values.sort();
    Ok(values)
}
