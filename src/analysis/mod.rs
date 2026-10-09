//! Optional inference infrastructure. No dependency from the playback worker.
mod jobs;
mod models;
mod worker;
pub use jobs::{AnalysisManager, JobStatus};
pub use worker::{Candidates, Detection, worker};
