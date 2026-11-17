//! Ephemeral maintainer measurements. Each elapsed interval belongs to one stage.
use serde::Serialize;
use std::time::Instant;
#[derive(Clone, Debug, Serialize)]
pub struct StageTiming {
    pub stage: String,
    pub seconds: f64,
}
pub(super) struct Timings {
    stage: String,
    since: Instant,
    intervals: Vec<StageTiming>,
    finished: bool,
}
impl Timings {
    pub fn new(since: Instant) -> Self {
        Self {
            stage: "job.start".into(),
            since,
            intervals: Vec::new(),
            finished: false,
        }
    }
    pub fn stage(&mut self, stage: &str) {
        if !self.finished && self.stage != stage {
            let now = Instant::now();
            self.intervals.push(StageTiming {
                stage: self.stage.clone(),
                seconds: now.duration_since(self.since).as_secs_f64(),
            });
            self.stage = stage.into();
            self.since = now;
        }
    }
    pub fn finish(&mut self) {
        if !self.finished {
            self.stage("job.finished");
            self.finished = true;
        }
    }
    pub fn snapshot(&self) -> Vec<StageTiming> {
        self.intervals.clone()
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn stages_partition_time_and_terminal_finalization_is_idempotent() {
        let mut timing = Timings::new(Instant::now());
        timing.stage("worker.load");
        timing.stage("worker.load");
        timing.stage("worker.infer");
        timing.finish();
        let result = timing.snapshot();
        timing.finish();
        timing.stage("unexpected");
        assert_eq!(result.len(), 3);
        assert_eq!(timing.snapshot().len(), 3);
        assert_eq!(
            result.iter().map(|s| s.stage.as_str()).collect::<Vec<_>>(),
            ["job.start", "worker.load", "worker.infer"]
        );
        assert!(result.iter().all(|s| s.seconds >= 0.));
    }
}
