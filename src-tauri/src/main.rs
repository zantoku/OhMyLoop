#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use ohmyloop::{
    desktop::{Action, Desktop, Snapshot, Transport},
    project::Document,
};
use tauri::{Manager, State};
use tauri_plugin_dialog::DialogExt;

struct KeymapPath(std::path::PathBuf);

#[tauri::command]
async fn preview_measure_map(
    song_key: String,
    map: Option<ohmyloop::timing::MeasureMap>,
    edit: ohmyloop::timing::Edit,
    state: State<'_, Desktop>,
) -> Result<ohmyloop::timing::Preview, String> {
    let desktop = state.inner().clone();
    work(move || {
        let audio = desktop.source(song_key)?;
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
    work(move || desktop.restore()).await
}

#[tauri::command]
async fn load_demo(state: State<'_, Desktop>) -> Result<Snapshot, String> {
    let desktop = state.inner().clone();
    work(move || desktop.load(None)).await
}

#[tauri::command]
async fn open_song(
    app: tauri::AppHandle,
    state: State<'_, Desktop>,
) -> Result<Option<Snapshot>, String> {
    let desktop = state.inner().clone();
    work(move || {
        let file = app
            .dialog()
            .file()
            .add_filter(
                "Songs",
                &["wav", "mp3", "flac", "ogg", "m4a", "aac", "aiff", "aif"],
            )
            .blocking_pick_file();
        file.map(|file| desktop.load(Some(file.into_path()?.to_string_lossy().into_owned())))
            .transpose()
    })
    .await
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
async fn transport_action(action: Action, state: State<'_, Desktop>) -> Result<Snapshot, String> {
    let desktop = state.inner().clone();
    work(move || desktop.act(action)).await
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
    song_key: String,
    revision: u64,
    desktop: State<'_, Desktop>,
    state: State<'_, ohmyloop::analysis::AnalysisManager>,
) -> Result<ohmyloop::analysis::JobStatus, String> {
    let desktop = desktop.inner().clone();
    let manager = state.inner().clone();
    work(move || {
        let audio = desktop.source(song_key.clone())?;
        manager.analyze(std::env::current_exe()?, audio, song_key, revision)
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
            app.manage(ohmyloop::analysis::AnalysisManager::new(models, cache)?);
            let desktop = Desktop::start(directory)?;
            if std::env::args().any(|arg| arg == "--demo") {
                desktop.load(None)?;
            }
            app.manage(desktop);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            read_keymap,
            analysis_status,
            install_analysis_model,
            cancel_analysis,
            start_analysis,
            preview_measure_map,
            measure_map_view,
            initialize,
            load_demo,
            open_song,
            update_document,
            transport_action,
            transport_status
        ])
        .on_window_event(|window, event| {
            if matches!(event, tauri::WindowEvent::CloseRequested { .. }) {
                window.app_handle().exit(0);
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
            tauri::RunEvent::Exit => {
                app.state::<ohmyloop::analysis::AnalysisManager>()
                    .shutdown();
                app.state::<Desktop>().shutdown();
            }
            _ => {}
        });
}
