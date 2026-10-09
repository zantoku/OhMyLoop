//! Optional inference infrastructure. No dependency from the playback worker.
mod inference_progress;
mod jobs;
mod models;
mod progress;
mod worker;
pub use jobs::{AnalysisManager, JobStatus};
pub use progress::{ProgressUnit, ProgressUpdate, WorkProgress};
pub use worker::{Candidates, Detection, worker};
