use super::control::Control;
pub use crate::stems::source::{Freshness, freshness};
use anyhow::Result;
use std::path::Path;
pub fn prepare(
    reference: &Path,
    owned: &Path,
    rate: u32,
    frames: u64,
    control: &Control,
    report: impl FnMut(&str, u64, Option<u64>),
) -> Result<(serde_json::Value, Freshness)> {
    crate::stems::source::prepare_reference(
        reference,
        owned,
        0.,
        None,
        Some((rate, frames)),
        control,
        report,
    )
}
