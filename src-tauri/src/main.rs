#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
mod lifecycle;
use ohmyloop::{
    desktop::{Action, Desktop, Snapshot, Transport},
    project::Document,
};
use tauri::{Emitter, Manager, State};
use tauri_plugin_dialog::DialogExt;

struct KeymapPath(std::path::PathBuf);

#[tauri::command]
async fn preview_measure_map(
    reference_token: String,
    map: Option<ohmyloop::timing::MeasureMap>,
    edit: ohmyloop::timing::Edit,
    state: State<'_, Desktop>,
) -> Result<ohmyloop::timing::Preview, String> {
    let desktop = state.inner().clone();
    work(move || {
        let audio = desktop.source(reference_token)?;
        let map = map.unwrap_or_else(|| {
            ohmyloop::timing::MeasureMap::empty(audio.rate, audio.frames.len() as u64)
        });
        anyhow::ensure!(
            map.rate == audio.rate && map.frames == audio.frames.len() as u64,
            "Musical map belongs to a different source"
        );
        map.edit(edit)
    })
    .await
}

#[tauri::command]
async fn measure_map_view(
    map: ohmyloop::timing::MeasureMap,
) -> Result<ohmyloop::timing::MapView, String> {
    work(move || {
        map.validate()?;
        Ok(map.view())
    })
    .await
}

async fn work<T: Send + 'static>(
    job: impl FnOnce() -> anyhow::Result<T> + Send + 'static,
) -> Result<T, String> {
    tauri::async_runtime::spawn_blocking(job)
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| format!("{e:#}"))
}

#[tauri::command]
async fn read_keymap(state: State<'_, KeymapPath>) -> Result<ohmyloop::keymap::KeymapFile, String> {
    let path = state.inner().0.clone();
    work(move || Ok(ohmyloop::keymap::read(&path))).await
}

#[tauri::command]
async fn initialize(state: State<'_, Desktop>) -> Result<Snapshot, String> {
    let desktop = state.inner().clone();
    work(move || desktop.restore_managed()).await
}

#[tauri::command]
async fn load_demo(state: State<'_, Desktop>) -> Result<Snapshot, String> {
    let desktop = state.inner().clone();
    work(move || desktop.load(None)).await
}

fn pick_recording(app: &tauri::AppHandle) -> anyhow::Result<Option<String>> {
    app.dialog()
        .file()
        .add_filter(
            "Songs",
            &["wav", "mp3", "flac", "ogg", "m4a", "aac", "aiff", "aif"],
        )
        .blocking_pick_file()
        .map(|file| Ok(file.into_path()?.to_string_lossy().into_owned()))
        .transpose()
}

#[tauri::command]
async fn library_status(
    state: State<'_, Desktop>,
) -> Result<ohmyloop::desktop::LibraryStatus, String> {
    let desktop = state.inner().clone();
    work(move || desktop.managed_status()).await
}
#[tauri::command]
async fn acknowledge_import_notice(state: State<'_, Desktop>) -> Result<(), String> {
    let desktop = state.inner().clone();
    work(move || desktop.acknowledge_import_notice()).await
}
#[tauri::command]
async fn choose_library(
    app: tauri::AppHandle,
    create: bool,
    locate: bool,
    state: State<'_, Desktop>,
) -> Result<Option<Snapshot>, String> {
    let desktop = state.inner().clone();
    work(move || {
        app.dialog()
            .file()
            .blocking_pick_folder()
            .map(|folder| {
                desktop.select_library(
                    folder.into_path()?.to_string_lossy().into_owned(),
                    create,
                    locate,
                )
            })
            .transpose()
    })
    .await
}
#[tauri::command]
async fn retry_library(state: State<'_, Desktop>) -> Result<Snapshot, String> {
    let desktop = state.inner().clone();
    work(move || desktop.retry_library()).await
}
#[derive(serde::Serialize)]
struct ImportSelection {
    path: String,
    candidates: Vec<ohmyloop::library::Entry>,
}
#[tauri::command]
async fn pick_import_song(
    app: tauri::AppHandle,
    state: State<'_, Desktop>,
) -> Result<Option<ImportSelection>, String> {
    let desktop = state.inner().clone();
    work(move || {
        pick_recording(&app)?
            .map(|path| {
                let candidates = desktop.import_candidates(path.clone())?;
                Ok(ImportSelection { path, candidates })
            })
            .transpose()
    })
    .await
}
#[derive(Clone, serde::Serialize)]
struct ImportProgress {
    copied: u64,
    total: u64,
}
#[tauri::command]
async fn import_song(
    app: tauri::AppHandle,
    path: String,
    state: State<'_, Desktop>,
) -> Result<Snapshot, String> {
    let desktop = state.inner().clone();
    work(move || {
        desktop.import_managed(path, |copied, total| {
            let _ = app.emit("import-progress", ImportProgress { copied, total });
        })
    })
    .await
}
#[tauri::command]
async fn delete_library_song(
    song_key: String,
    state: State<'_, Desktop>,
) -> Result<Snapshot, String> {
    let desktop = state.inner().clone();
    work(move || desktop.delete_library_song(song_key)).await
}

#[tauri::command]
async fn replace_recording(
    app: tauri::AppHandle,
    reference_token: String,
    timing_unchanged: bool,
    state: State<'_, Desktop>,
) -> Result<Option<Snapshot>, String> {
    let desktop = state.inner().clone();
    work(move || {
        pick_recording(&app)?
            .map(|path| desktop.replace_reference(reference_token, path, timing_unchanged))
            .transpose()
    })
    .await
}

#[tauri::command]
async fn library_reference(song_key: String, state: State<'_, Desktop>) -> Result<String, String> {
    let desktop = state.inner().clone();
    work(move || desktop.library_reference(song_key)).await
}
#[tauri::command]
async fn replace_library_reference(
    app: tauri::AppHandle,
    song_key: String,
    reference_token: String,
    timing_unchanged: bool,
    state: State<'_, Desktop>,
) -> Result<Option<Snapshot>, String> {
    let desktop = state.inner().clone();
    work(move || {
        pick_recording(&app)?
            .map(|path| {
                desktop.replace_library_reference(song_key, reference_token, path, timing_unchanged)
            })
            .transpose()
    })
    .await
}

#[tauri::command]
async fn read_library(state: State<'_, Desktop>) -> Result<ohmyloop::library::Catalogue, String> {
    let desktop = state.inner().clone();
    work(move || {
        if desktop.managed_status()?.selected.is_none() {
            Ok(ohmyloop::library::Catalogue::default())
        } else {
            desktop.library()
        }
    })
    .await
}
#[tauri::command]
async fn open_library_song(
    song_key: String,
    state: State<'_, Desktop>,
) -> Result<Snapshot, String> {
    let desktop = state.inner().clone();
    work(move || desktop.open_library(song_key)).await
}
#[tauri::command]
async fn update_library_song(
    song_key: String,
    metadata: ohmyloop::library::Metadata,
    state: State<'_, Desktop>,
) -> Result<ohmyloop::library::Catalogue, String> {
    let desktop = state.inner().clone();
    work(move || desktop.update_library(song_key, metadata)).await
}
#[tauri::command]
async fn update_document(
    document: Document,
    state: State<'_, Desktop>,
) -> Result<Snapshot, String> {
    let desktop = state.inner().clone();
    work(move || desktop.update(document)).await
}

#[tauri::command]
async fn transport_action(
    action: Action,
    reference_token: String,
    state: State<'_, Desktop>,
) -> Result<Snapshot, String> {
    let desktop = state.inner().clone();
    work(move || desktop.act(action, reference_token)).await
}

#[tauri::command]
async fn set_map_metronome(
    reference_token: String,
    enabled: bool,
    volume: f32,
    map: Option<ohmyloop::timing::MeasureMap>,
    state: State<'_, Desktop>,
) -> Result<Transport, String> {
    let desktop = state.inner().clone();
    work(move || desktop.metronome(reference_token, enabled, volume, map)).await
}

#[tauri::command]
async fn set_count_in(
    reference_token: String,
    enabled: bool,
    repeat: bool,
    volume: f32,
    state: State<'_, Desktop>,
) -> Result<Transport, String> {
    let desktop = state.inner().clone();
    work(move || desktop.count_in(reference_token, enabled, repeat, volume)).await
}

#[tauri::command]
async fn transport_status(state: State<'_, Desktop>) -> Result<Transport, String> {
    let desktop = state.inner().clone();
    work(move || desktop.status()).await
}

#[tauri::command]
async fn analysis_status(
    state: State<'_, ohmyloop::analysis::AnalysisManager>,
) -> Result<ohmyloop::analysis::JobStatus, String> {
    Ok(state.status())
}
#[tauri::command]
async fn read_cached_analysis(
    reference_token: String,
    desktop: State<'_, Desktop>,
    state: State<'_, ohmyloop::analysis::AnalysisManager>,
) -> Result<Option<ohmyloop::analysis::Candidates>, String> {
    let desktop = desktop.inner().clone();
    let manager = state.inner().clone();
    work(move || {
        let audio = desktop.source(reference_token.clone())?;
        manager.cached_result(&reference_token, &audio)
    })
    .await
}
#[tauri::command]
async fn install_analysis_model(
    state: State<'_, ohmyloop::analysis::AnalysisManager>,
) -> Result<ohmyloop::analysis::JobStatus, String> {
    state.install().map_err(|e| e.to_string())
}
#[tauri::command]
async fn cancel_analysis(
    id: u64,
    state: State<'_, ohmyloop::analysis::AnalysisManager>,
) -> Result<ohmyloop::analysis::JobStatus, String> {
    Ok(state.cancel(id))
}
#[tauri::command]
async fn start_analysis(
    reference_token: String,
    revision: u64,
    desktop: State<'_, Desktop>,
    state: State<'_, ohmyloop::analysis::AnalysisManager>,
) -> Result<ohmyloop::analysis::JobStatus, String> {
    let desktop = desktop.inner().clone();
    let manager = state.inner().clone();
    work(move || {
        let audio = desktop.source(reference_token.clone())?;
        manager.analyze(std::env::current_exe()?, audio, reference_token, revision)
    })
    .await
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.first().is_some_and(|arg| arg == "--analysis-worker") {
        if let Err(error) = ohmyloop::analysis::worker(&args[1..]) {
            eprintln!("{error:#}");
            std::process::exit(1);
        }
        return;
    }
    tauri::Builder::default()
        .manage(lifecycle::ExitGate::default())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let keymap = std::env::var_os("OHMYLOOP_KEYMAP_FILE")
                .map(std::path::PathBuf::from)
                .unwrap_or(app.path().app_config_dir()?.join("keymap.json"));
            app.manage(KeymapPath(keymap));
            let directory = std::env::var_os("OHMYLOOP_DATA_DIR")
                .map(std::path::PathBuf::from)
                .unwrap_or(app.path().app_data_dir()?.join("practice"));
            let models = std::env::var_os("OHMYLOOP_MODEL_DIR")
                .map(std::path::PathBuf::from)
                .unwrap_or(app.path().app_data_dir()?.join("models/beat-this-v1"));
            let cache = directory.parent().unwrap_or(&directory).join("analysis");
            let desktop = Desktop::start(directory)?;
            app.manage(ohmyloop::analysis::AnalysisManager::new(models, cache)?);
            if std::env::args().any(|arg| arg == "--demo") {
                desktop.load(None)?;
            }
            app.manage(desktop);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            lifecycle::finish_exit,
            read_keymap,
            analysis_status,
            read_cached_analysis,
            install_analysis_model,
            cancel_analysis,
            start_analysis,
            preview_measure_map,
            measure_map_view,
            initialize,
            load_demo,
            library_status,
            acknowledge_import_notice,
            choose_library,
            retry_library,
            pick_import_song,
            import_song,
            delete_library_song,
            replace_recording,
            library_reference,
            replace_library_reference,
            read_library,
            open_library_song,
            update_library_song,
            update_document,
            transport_action,
            set_map_metronome,
            set_count_in,
            transport_status
        ])
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                lifecycle::request(window.app_handle());
            }
        })
        .build(tauri::generate_context!())
        .expect("Could not start OhMyLoop")
        .run(|app, event| match event {
            tauri::RunEvent::Ready => {
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.show();
                    let _ = window.set_focus();
                }
            }
            #[cfg(target_os = "macos")]
            tauri::RunEvent::Reopen { .. } => {
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.show();
                    let _ = window.unminimize();
                    let _ = window.set_focus();
                }
            }
            tauri::RunEvent::ExitRequested { api, .. } => {
                if !app.state::<lifecycle::ExitGate>().approved() {
                    api.prevent_exit();
                    lifecycle::request(app);
                }
            }
            tauri::RunEvent::Exit => {
                app.state::<ohmyloop::analysis::AnalysisManager>()
                    .shutdown();
                app.state::<Desktop>().shutdown();
            }
            _ => {}
        });
}
