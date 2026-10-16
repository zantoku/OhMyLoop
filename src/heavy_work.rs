//! A single admission slot for expensive timing analysis and stem inference.
use anyhow::{Result, ensure};
use std::sync::{Arc, Mutex};
#[derive(Clone, Default)]
pub struct HeavyWork(Arc<Mutex<Option<&'static str>>>);
pub struct Permit(HeavyWork);
impl HeavyWork {
    pub fn acquire(&self, kind: &'static str) -> Result<Permit> {
        let mut slot = self.0.lock().unwrap();
        ensure!(
            slot.is_none(),
            "{} is already running; wait for it to finish",
            slot.unwrap_or("Heavy work")
        );
        *slot = Some(kind);
        Ok(Permit(self.clone()))
    }
    pub fn active(&self) -> Option<&'static str> {
        *self.0.lock().unwrap()
    }
}
impl Drop for Permit {
    fn drop(&mut self) {
        *self.0.0.lock().unwrap() = None;
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn admission_preserves_incumbent_until_cleanup() {
        let gate = HeavyWork::default();
        let permit = gate.acquire("Timing analysis").unwrap();
        assert!(gate.acquire("Stem separation").is_err());
        assert_eq!(gate.active(), Some("Timing analysis"));
        drop(permit);
        assert!(gate.acquire("Stem separation").is_ok());
    }
}
