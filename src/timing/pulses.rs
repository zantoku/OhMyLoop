//! Musical pulse events derived only from the authored map, not analysis evidence.
use super::*;
use anyhow::{Result, ensure};
use std::collections::HashMap;
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Accent {
    Beat,
    Group,
    Measure,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PulseEvent {
    pub frame: u64,
    pub accent: Accent,
}
impl MeasureMap {
    pub fn pulse_events(&self) -> Result<Vec<PulseEvent>> {
        self.validate()?;
        let boundaries: HashMap<_, _> = self
            .boundaries
            .iter()
            .map(|b| (b.id.as_str(), b.frame))
            .collect();
        let mut events = vec![];
        for measure in &self.measures {
            let pulse = measure.meter.pulse.value();
            let origin = measure.offset.value();
            let count = (measure.meter.length().value() / pulse).ceil() as usize;
            ensure!(
                count <= 512,
                "Choose a larger pulse note value for the metronome"
            );
            let mut groups = vec![];
            let mut at = 0.0;
            for group in &measure.meter.grouping {
                groups.push(at);
                at += f64::from(*group) * 4.0 / f64::from(measure.meter.denominator);
            }
            let (start, end) = (
                boundaries[measure.start.as_str()],
                boundaries[measure.end.as_str()],
            );
            for index in 0..count {
                let absolute = index as f64 * pulse;
                let quarter = absolute - origin;
                if quarter < -1e-9 || quarter >= measure.length.value() - 1e-9 {
                    continue;
                }
                let frame = Self::frame_between(measure, start, end, quarter.max(0.0));
                if frame >= end {
                    continue;
                }
                let accent = if absolute.abs() < 1e-9 {
                    Accent::Measure
                } else if groups.iter().any(|g| (g - absolute).abs() < 1e-9) {
                    Accent::Group
                } else {
                    Accent::Beat
                };
                events.push(PulseEvent { frame, accent });
                ensure!(
                    events.len() <= MAX_BEATS,
                    "The metronome map contains too many pulses"
                );
            }
        }
        // Rounding tiny intervals may put multiple pulses on one source frame.
        events.sort_by_key(|e| e.frame);
        events.dedup_by(|later, earlier| {
            if later.frame == earlier.frame {
                earlier.accent = earlier.accent.max(later.accent);
                true
            } else {
                false
            }
        });
        Ok(events)
    }
}
