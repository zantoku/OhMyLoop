//! Cooperative cancellation and the serialized publication boundary.
use anyhow::{Result, ensure};
use std::sync::{Arc, Mutex};
#[derive(Clone, Default)]
pub struct Control(Arc<Mutex<State>>);
#[derive(Default)]
struct State {
    cancelled: bool,
    committing: bool,
}
impl Control {
    pub fn check(&self) -> Result<()> {
        ensure!(!self.cancelled(), "Cancelled");
        Ok(())
    }
    pub fn cancelled(&self) -> bool {
        self.0.lock().unwrap().cancelled
    }
    pub fn cancel(&self) -> bool {
        let mut s = self.0.lock().unwrap();
        if s.committing || s.cancelled {
            return false;
        }
        s.cancelled = true;
        true
    }
    pub fn commit(&self) -> Result<()> {
        let mut s = self.0.lock().unwrap();
        ensure!(!s.cancelled, "Cancelled");
        s.committing = true;
        Ok(())
    }
    pub fn committing(&self) -> bool {
        self.0.lock().unwrap().committing
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cancellation_and_publication_are_exclusive() {
        let c = Control::default();
        assert!(c.cancel());
        assert!(c.commit().is_err());
        let c = Control::default();
        c.commit().unwrap();
        assert!(!c.cancel());
        assert!(!c.cancelled());
    }
    #[test]
    fn concurrent_cancel_and_commit_have_one_winner() {
        for _ in 0..100 {
            let control = Control::default();
            let barrier = Arc::new(std::sync::Barrier::new(3));
            let c = control.clone();
            let b = barrier.clone();
            let cancel = std::thread::spawn(move || {
                b.wait();
                c.cancel()
            });
            let c = control.clone();
            let b = barrier.clone();
            let commit = std::thread::spawn(move || {
                b.wait();
                c.commit().is_ok()
            });
            barrier.wait();
            let cancelled = cancel.join().unwrap();
            let committing = commit.join().unwrap();
            assert_ne!(cancelled, committing);
            assert_eq!(control.cancelled(), cancelled);
            assert_eq!(control.committing(), committing);
        }
    }
}
