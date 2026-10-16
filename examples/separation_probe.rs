//! Maintainer QA using the production desktop/separation APIs and isolated storage.
use anyhow::{Context, Result, ensure};
use ohmyloop::{
    desktop::{Action, Desktop},
    heavy_work::HeavyWork,
    separation::SeparationManager,
};
use std::{
    fs,
    path::PathBuf,
    thread,
    time::{Duration, Instant},
};
fn main() -> Result<()> {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    ensure!(
        args.len() >= 3,
        "Usage: separation_probe install|run|remove SOURCE ISOLATED_DIRECTORY [CANCEL_STAGE]"
    );
    let mode = &args[0];
    let source = PathBuf::from(&args[1]);
    let root = std::path::absolute(&args[2])?;
    fs::create_dir_all(&root)?;
    // Optional read-only reuse of an installed package; no install/remove allowed.
    let existing_models = std::env::var_os("OHMYLOOP_PROBE_MODELS").map(PathBuf::from);
    ensure!(
        existing_models.is_none() || mode == "run",
        "An external model path is read-only and only supports run"
    );
    let manager = SeparationManager::new(
        existing_models.unwrap_or_else(|| root.join("app-models")),
        HeavyWork::default(),
    );
    println!(
        "runtime/package status {}",
        serde_json::to_string(&manager.status())?
    );
    if mode == "remove" {
        manager.remove_model()?;
        let pointer = root.join("library-root.txt");
        if pointer.exists() {
            let desktop = Desktop::start(root.join("practice"))?;
            desktop.select_library(fs::read_to_string(pointer)?, false, false)?;
            let candidates = desktop.import_candidates(source.display().to_string())?;
            let song = desktop
                .open_library(
                    candidates
                        .first()
                        .context("Missing QA song")?
                        .song_key
                        .clone(),
                )?
                .song
                .context("Missing QA song")?;
            let stems = desktop.stems_status(song.reference_token)?;
            ensure!(
                stems.available && stems.set_id.is_some(),
                "Removing model made completed stems unavailable"
            );
            ensure!(!desktop.status()?.playing, "Reopening should remain paused");
            println!(
                "model-independent playback {}",
                serde_json::to_string(&stems)?
            );
            desktop.shutdown();
        }
        manager.shutdown();
        return Ok(());
    }
    if mode == "install" {
        manager.install()?;
        let mut phase = String::new();
        loop {
            let s = manager.status();
            if s.install.phase != phase {
                phase = s.install.phase.clone();
                println!("install {}", serde_json::to_string(&s.install)?);
            }
            if !s.install.running {
                ensure!(
                    s.install.error.is_none(),
                    "{}",
                    s.install.error.unwrap_or_default()
                );
                break;
            }
            thread::sleep(Duration::from_millis(100));
        }
        manager.shutdown();
        return Ok(());
    }
    ensure!(mode == "run", "Unknown mode");
    let desktop = Desktop::start(root.join("practice"))?;
    let library_root = root.join("library");
    fs::create_dir_all(&library_root)?;
    let pointer = root.join("library-root.txt");
    if pointer.exists() {
        desktop.select_library(fs::read_to_string(&pointer)?, false, false)?;
    } else {
        let s = desktop.select_library(library_root.display().to_string(), true, false)?;
        fs::write(
            &pointer,
            s.library_status
                .selected
                .context("Missing library pointer")?
                .root,
        )?;
    }
    let candidates = desktop.import_candidates(source.display().to_string())?;
    let snapshot = if let Some(song) = candidates.first() {
        desktop.open_library(song.song_key.clone())?
    } else {
        desktop.import_managed(source.display().to_string(), |_, _| {})?
    };
    let song = snapshot.song.context("No song")?;
    let mut document = snapshot.document.context("No document")?;
    let previous = desktop.stems_status(song.reference_token.clone())?.set_id;
    let begin = Instant::now();
    manager.start(desktop.clone(), song.reference_token.clone())?;
    ensure!(
        desktop
            .act(Action::Seek { seconds: 1. }, song.reference_token.clone())
            .is_err(),
        "Seek was not frozen"
    );
    let cancel_stage = args.get(3);
    let mut cancelled = false;
    let mut edited = false;
    let mut last = String::new();
    loop {
        let s = manager.status();
        let job = &s.job;
        let stage = job
            .progress
            .as_ref()
            .map(|p| p.stage.as_str())
            .unwrap_or("");
        let signature = format!("{}:{stage}", job.phase);
        if signature != last {
            last = signature;
            println!("job {}", serde_json::to_string(job)?);
        }
        if job.running && !edited && stage == "worker.infer" {
            document.notes = "QA note saved during separation".into();
            desktop.update(document.clone())?;
            edited = true;
        }
        if job.running
            && !cancelled
            && cancel_stage.is_some_and(|v| stage.starts_with(v) || job.phase.starts_with(v))
        {
            println!("request cancel at {stage}");
            manager.cancel(&job.id);
            cancelled = true;
        }
        if !job.running {
            println!(
                "terminal elapsed {:.3}s {}",
                begin.elapsed().as_secs_f64(),
                serde_json::to_string(job)?
            );
            println!(
                "stage timings {}",
                serde_json::to_string(&manager.timings())?
            );
            if !cancelled {
                ensure!(
                    job.phase == "Complete",
                    "{}",
                    job.error.clone().unwrap_or_else(|| job.phase.clone())
                );
            }
            break;
        }
        thread::sleep(Duration::from_millis(100));
    }
    let stems = desktop.stems_status(song.reference_token.clone())?;
    let library = ohmyloop::managed::ManagedLibrary::open(std::path::Path::new(
        &fs::read_to_string(&pointer)?,
    ))?;
    let final_document = library.record(&song.key)?.record;
    ensure!(
        !edited || final_document.notes == "QA note saved during separation",
        "Concurrent notes were lost"
    );
    ensure!(
        final_document.timeline.measure_map == document.measure_map,
        "Authored map was changed"
    );
    if cancelled {
        ensure!(
            stems.set_id == previous,
            "Cancelled job changed association"
        );
    }
    println!("final stems {}", serde_json::to_string(&stems)?);
    println!("transport {}", serde_json::to_string(&desktop.status()?)?);
    manager.shutdown();
    desktop.shutdown();
    Ok(())
}
