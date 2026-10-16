//! Give the UI a chance to flush pending edits before a normal window/app exit.
use std::sync::atomic::{AtomicBool, Ordering};
use tauri::{Emitter, Manager};
#[derive(Default)]
pub struct ExitGate(AtomicBool);
impl ExitGate {
    pub fn approved(&self) -> bool {
        self.0.load(Ordering::Acquire)
    }
}
#[derive(Clone, serde::Serialize)]
struct ExitRequest {
    active_processing: bool,
}
pub fn request(app: &tauri::AppHandle) {
    let active_processing = app
        .try_state::<ohmyloop::separation::SeparationManager>()
        .is_some_and(|manager| manager.active());
    let _ = app.emit("prepare-exit", ExitRequest { active_processing });
}
#[tauri::command]
pub async fn finish_exit(app: tauri::AppHandle) -> Result<(), String> {
    let manager = app
        .state::<ohmyloop::separation::SeparationManager>()
        .inner()
        .clone();
    tauri::async_runtime::spawn_blocking(move || manager.shutdown())
        .await
        .map_err(|error| error.to_string())?;
    app.state::<ExitGate>().0.store(true, Ordering::Release);
    app.exit(0);
    Ok(())
}
