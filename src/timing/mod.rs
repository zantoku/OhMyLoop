//! Accepted musical definitions, independent of playback and analysis backends.
mod edit;
mod model;
mod projection;
mod pulses;
mod validation;
pub use edit::Edit;
pub use model::*;
pub use projection::*;
pub use pulses::{Accent, PulseEvent};
const MAX_MEASURES: usize = 10_000;
const MAX_BEATS: usize = 80_000;
const MAX_SAFE_INTEGER: u64 = (1 << 53) - 1;
