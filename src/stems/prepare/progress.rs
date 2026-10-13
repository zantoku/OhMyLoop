//! Desktop task progress; diagnostic phase text remains available to CLI callers.
use super::*;
use serde::Serialize;

#[derive(Clone, Debug, Serialize)]
pub struct WorkflowStep {
    pub id: String,
    pub label: String,
    pub state: &'static str,
    pub completed: u64,
    pub total: Option<u64>,
}
#[derive(Clone, Debug, Serialize)]
pub struct WorkflowProgress {
    pub steps: Vec<WorkflowStep>,
    /// Completed task units, including the active task's measured fraction.
    pub completed: f64,
    pub total: usize,
}
pub struct WorkflowTracker {
    steps: Vec<WorkflowStep>,
    current: Option<usize>,
}
impl WorkflowTracker {
    fn new(plan: Vec<(String, String)>) -> Self {
        Self {
            steps: plan
                .into_iter()
                .map(|(id, label)| WorkflowStep {
                    id,
                    label,
                    state: "pending",
                    completed: 0,
                    total: None,
                })
                .collect(),
            current: None,
        }
    }
    pub fn attachment(prototype: &Path) -> Result<Self> {
        let path = safe_file(prototype, "manifest.json")?;
        ensure!(
            fs::metadata(&path)?.len() <= 4 * 1024 * 1024,
            "Oversized prototype manifest"
        );
        let manifest: Manifest = serde_json::from_slice(&fs::read(path)?)?;
        let roles = manifest.stems.keys().cloned().collect::<Vec<_>>();
        for role in &roles {
            Role::parse(role)?;
        }
        let mut plan = Vec::new();
        for role in &roles {
            plan.push((format!("owned.{role}.copy"), format!("Copy {role} audio")));
        }
        plan.extend(vec![
            (
                "validation.reference.hash".into(),
                "Check reference recording".into(),
            ),
            (
                "validation.reference.decode".into(),
                "Read reference recording".into(),
            ),
            (
                "validation.reference.resample".into(),
                "Align reference playback".into(),
            ),
            (
                "validation.prepared.hash".into(),
                "Check separation source".into(),
            ),
            (
                "validation.prepared.decode".into(),
                "Read separation source".into(),
            ),
        ]);
        for role in &roles {
            for (stage, action) in [("hash", "Check"), ("decode", "Read"), ("resample", "Align")] {
                plan.push((
                    format!("validation.{role}.{stage}"),
                    format!("{action} {role} audio"),
                ));
            }
        }
        plan.push((
            "validation.headroom".into(),
            "Measure safe playback levels".into(),
        ));
        plan.push(("cache.adopt".into(), "Keep validated playback audio".into()));
        plan.push(("cache.save".into(), "Save prepared playback".into()));
        plan.push(("attachment.publish".into(), "Save attached stems".into()));
        Ok(Self::new(plan))
    }
    fn cache_plan(roles: &[String]) -> Vec<(String, String)> {
        let mut plan = vec![
            (
                "cache.reference.decode".into(),
                "Prepare original playback".into(),
            ),
            (
                "cache.reference.resample".into(),
                "Align original playback".into(),
            ),
        ];
        for role in roles {
            plan.push((
                format!("cache.{role}.decode"),
                format!("Prepare {role} playback"),
            ));
            plan.push((
                format!("cache.{role}.resample"),
                format!("Align {role} playback"),
            ));
        }
        plan.push(("cache.headroom".into(), "Set safe playback volume".into()));
        plan.push(("cache.save".into(), "Save prepared playback".into()));
        plan
    }
    pub(super) fn reuse() -> Self {
        Self::new(vec![(
            "cache.reuse".into(),
            "Reuse prepared playback".into(),
        )])
    }
    pub(super) fn cache(roles: &[String]) -> Self {
        Self::new(Self::cache_plan(roles))
    }
    pub fn begin_first(&mut self) -> PreparationProgress {
        let id = self.steps.first().map(|s| s.id.clone()).unwrap_or_default();
        self.begin(&id)
    }
    pub fn begin(&mut self, id: &str) -> PreparationProgress {
        self.update(PreparationProgress {
            phase: "Processing audio".into(),
            completed: 0,
            total: None,
            step: Some(id.into()),
            workflow: None,
        })
    }
    pub fn finish(&mut self) -> PreparationProgress {
        for step in &mut self.steps {
            if step.state != "skipped" {
                step.state = "complete";
            }
            step.completed = 1;
            step.total = Some(1);
        }
        let mut p = PreparationProgress {
            phase: "Processing complete".into(),
            completed: 1,
            total: Some(1),
            step: None,
            workflow: None,
        };
        p.workflow = Some(self.snapshot());
        p
    }
    pub fn update(&mut self, mut progress: PreparationProgress) -> PreparationProgress {
        if let Some(role) = progress
            .step
            .as_deref()
            .and_then(|id| id.strip_prefix("skip."))
            && Role::parse(role).is_ok()
            && let Some(index) = self
                .steps
                .iter()
                .position(|s| s.id == format!("validation.{role}.resample"))
        {
            for step in &mut self.steps[..index] {
                if step.state != "skipped" {
                    step.state = "complete";
                    step.completed = 1;
                    step.total = Some(1);
                }
            }
            let step = &mut self.steps[index];
            step.state = "skipped";
            step.label = format!("Skip near-silent {role}");
            step.completed = 1;
            step.total = Some(1);
            self.current = Some(index);
            progress.workflow = Some(self.snapshot());
            return progress;
        }
        if let Some(index) = progress
            .step
            .as_ref()
            .and_then(|id| self.steps.iter().position(|s| &s.id == id))
        {
            if self.current != Some(index) {
                // Only successful sequential handoffs complete earlier work.
                for step in &mut self.steps[..index] {
                    if step.state != "skipped" {
                        step.state = "complete";
                    }
                    step.completed = 1;
                    step.total = Some(1);
                }
                self.current = Some(index);
            }
            let step = &mut self.steps[index];
            step.state = "active";
            step.completed = progress.completed;
            step.total = progress.total;
        }
        progress.workflow = Some(self.snapshot());
        progress
    }
    fn snapshot(&self) -> WorkflowProgress {
        let completed = self
            .steps
            .iter()
            .map(|step| match step.state {
                "complete" | "skipped" => 1.0,
                "active" => step
                    .total
                    .filter(|n| *n > 0)
                    .map(|n| (step.completed as f64 / n as f64).clamp(0.0, 1.0))
                    .unwrap_or(0.0),
                _ => 0.0,
            })
            .sum();
        WorkflowProgress {
            steps: self.steps.clone(),
            completed,
            total: self.steps.len(),
        }
    }
}

/// Rate-limit UI reports while preserving semantic transitions and completion.
pub fn throttled_progress(
    mut report: impl FnMut(PreparationProgress),
) -> impl FnMut(PreparationProgress) {
    let started = Instant::now();
    let mut gate = ProgressGate::default();
    move |event| {
        if gate.accept(&event, started.elapsed()) {
            report(event);
        }
    }
}
#[derive(Default)]
struct ProgressGate {
    emitted_at: Option<Duration>,
    step: Option<String>,
    complete: bool,
}
impl ProgressGate {
    fn accept(&mut self, event: &PreparationProgress, now: Duration) -> bool {
        let changed = self.step != event.step;
        let complete = event
            .total
            .is_some_and(|total| total > 0 && event.completed >= total);
        if self
            .emitted_at
            .is_none_or(|last| now.saturating_sub(last) >= Duration::from_millis(100))
            || changed
            || (complete && (!self.complete || changed))
        {
            self.emitted_at = Some(now);
            self.step = event.step.clone();
            self.complete = complete;
            true
        } else {
            false
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn throttling_keeps_stage_and_completion_events_without_packet_flooding() {
        let mut gate = ProgressGate::default();
        let mut event = PreparationProgress {
            phase: "Read audio".into(),
            completed: 0,
            total: Some(100),
            step: Some("read".into()),
            workflow: None,
        };
        assert!(gate.accept(&event, Duration::ZERO));
        for n in 1..100 {
            event.completed = n;
            assert!(!gate.accept(&event, Duration::from_millis(n)));
        }
        assert!(gate.accept(&event, Duration::from_millis(100)));
        event.completed = 100;
        assert!(gate.accept(&event, Duration::from_millis(101)));
        assert!(!gate.accept(&event, Duration::from_millis(102)));
        event.step = Some("save".into());
        event.completed = 0;
        event.total = None;
        assert!(gate.accept(&event, Duration::from_millis(103)));
        event.step = None;
        event.completed = 1;
        event.total = Some(1);
        assert!(gate.accept(&event, Duration::from_millis(104)));
    }
}
