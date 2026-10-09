//! Bounded worker progress protocol, separate from detection results.
use serde::{Deserialize, Serialize};
use std::io::{BufRead, BufReader, Read};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ProgressUnit {
    Bytes,
    Frames,
    Segments,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct WorkProgress {
    pub completed: u64,
    pub total: Option<u64>,
    pub unit: ProgressUnit,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProgressUpdate {
    pub phase: String,
    pub progress: Option<WorkProgress>,
}
impl ProgressUpdate {
    pub fn stage(phase: &str) -> Self {
        Self {
            phase: phase.into(),
            progress: None,
        }
    }
    pub fn measured(phase: &str, completed: u64, total: Option<u64>, unit: ProgressUnit) -> Self {
        Self {
            phase: phase.into(),
            progress: Some(WorkProgress {
                completed,
                total,
                unit,
            }),
        }
    }
    fn valid(&self) -> bool {
        !self.phase.is_empty()
            && self.phase.len() <= 96
            && self.progress.as_ref().is_none_or(|p| {
                p.total
                    .is_none_or(|total| total > 0 && p.completed <= total)
            })
    }
}
pub fn read_updates(reader: impl Read, mut publish: impl FnMut(ProgressUpdate)) {
    let mut reader = BufReader::new(reader);
    loop {
        let mut line = String::new();
        // A corrupt or unrelated worker cannot grow an unbounded line buffer.
        let Ok(size) = (&mut reader).take(1024).read_line(&mut line) else {
            break;
        };
        if size == 0 || !line.ends_with('\n') {
            break;
        }
        if let Ok(update) = serde_json::from_str::<ProgressUpdate>(&line)
            && update.valid()
        {
            publish(update);
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn worker_protocol_accepts_only_bounded_valid_updates() {
        let valid = ProgressUpdate::measured("Finding beats", 2, Some(5), ProgressUnit::Segments);
        let line = format!("{}\n", serde_json::to_string(&valid).unwrap());
        let invalid = "{\"phase\":\"Bad\",\"progress\":{\"completed\":6,\"total\":5,\"unit\":\"segments\"}}\n";
        let input = format!("garbage\n{invalid}{line}{}\n{line}", "x".repeat(2048));
        let mut updates = vec![];
        read_updates(input.as_bytes(), |u| updates.push(u));
        assert_eq!(updates.len(), 1);
        assert_eq!(updates[0].progress, valid.progress);
    }
}
