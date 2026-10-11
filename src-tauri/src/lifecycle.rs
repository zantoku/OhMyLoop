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
pub fn request(app: &tauri::AppHandle) {
    let _ = app.emit("prepare-exit", ());
}
#[tauri::command]
pub fn finish_exit(app: tauri::AppHandle) {
    app.state::<ExitGate>().0.store(true, Ordering::Release);
    app.exit(0);
}
