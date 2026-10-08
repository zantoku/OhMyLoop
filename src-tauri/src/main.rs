#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use ohmyloop::{
    desktop::{Action, Desktop, Snapshot, Transport},
    project::Document,
};
use tauri::{Manager, State};
use tauri_plugin_dialog::DialogExt;

async fn work<T: Send + 'static>(
    job: impl FnOnce() -> anyhow::Result<T> + Send + 'static,
) -> Result<T, String> {
    tauri::async_runtime::spawn_blocking(job)
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| format!("{e:#}"))
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

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let directory = std::env::var_os("OHMYLOOP_DATA_DIR")
                .map(std::path::PathBuf::from)
                .unwrap_or(app.path().app_data_dir()?.join("practice"));
            let desktop = Desktop::start(directory)?;
            if std::env::args().any(|arg| arg == "--demo") {
                desktop.load(None)?;
            }
            app.manage(desktop);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
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
            tauri::RunEvent::Exit => app.state::<Desktop>().shutdown(),
            _ => {}
        });
}
