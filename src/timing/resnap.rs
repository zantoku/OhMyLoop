//! Opt-in refitting of shifted measures to analysis evidence.
use super::fitting::{default_beat_epsilon_ms, fitted_anchor_indices_at};
use super::*;
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResnapOptions {
    pub pulses: Vec<u64>,
    #[serde(default = "default_beat_epsilon_ms")]
    pub epsilon_ms: f64,
}

fn nearest(pulses: &[u64], frame: u64) -> u64 {
    let i = pulses.partition_point(|p| *p < frame);
    match (i.checked_sub(1).map(|i| pulses[i]), pulses.get(i)) {
        (Some(left), Some(&right)) if frame - left <= right - frame => left,
        (_, Some(&right)) => right,
        (Some(left), None) => left,
        _ => unreachable!("Validated nonempty detection"),
    }
}

impl MeasureMap {
    pub(super) fn resnap_suffix(&mut self, first: usize, options: ResnapOptions) -> Result<()> {
        let ResnapOptions { pulses, epsilon_ms } = options;
        ensure!(
            epsilon_ms.is_finite() && (0.0..=1000.0).contains(&epsilon_ms),
            "Beat timing tolerance must be between 0 and 1000 ms"
        );
        ensure!(
            pulses.len() >= 2
                && pulses.len() <= MAX_BEATS
                && pulses.windows(2).all(|w| w[0] < w[1])
                && pulses.iter().all(|p| *p < self.frames),
            "Invalid detected pulse positions"
        );
        let (start, _) = self.bounds(&self.measures[first]);
        let (_, end) = self.bounds(self.measures.last().unwrap());
        // Only internal cuts move; the unchanged prefix and free-time edges stay intact.
        let ids: Vec<_> = self.measures[first..]
            .iter()
            .skip(1)
            .map(|m| m.start.clone())
            .collect();
        let mut previous = start;
        for id in ids {
            let boundary = self.boundaries.iter_mut().find(|b| b.id == id).unwrap();
            let frame = nearest(&pulses, boundary.frame);
            ensure!(
                frame > previous && frame < end,
                "Re-snap would collapse a measure; review detected beats or turn re-snap off"
            );
            boundary.frame = frame;
            previous = frame;
        }
        for i in first..self.measures.len() {
            let measure = &self.measures[i];
            let (a, b) = self.bounds(measure);
            let pulse = measure.meter.pulse;
            let count = (measure.meter.length().value() / pulse.value()).ceil() as u32;
            ensure!(
                count <= 512,
                "Choose a larger pulse note value for re-snapping"
            );
            let mut offsets = vec![Fraction::new(0, 1)];
            for k in 0..count {
                let absolute = Fraction::new(pulse.num * k, pulse.den);
                if absolute.value() > measure.offset.value() {
                    let offset = absolute.sub(measure.offset)?;
                    if offset.value() < measure.length.value() {
                        offsets.push(offset);
                    }
                }
            }
            offsets.push(measure.length);
            let lo = pulses.partition_point(|p| *p <= a);
            let hi = pulses.partition_point(|p| *p < b);
            let inside = &pulses[lo..hi];
            ensure!(
                inside.len() == offsets.len() - 2,
                "Re-snap pulse count disagrees with measure {} ({} detected, {} expected inside); review detection or turn re-snap off",
                i + 1,
                inside.len(),
                offsets.len() - 2
            );
            let points: Vec<_> = std::iter::once(a)
                .chain(inside.iter().copied())
                .chain([b])
                .collect();
            let positions: Vec<_> = offsets.iter().map(|q| q.value()).collect();
            let fitted = fitted_anchor_indices_at(
                &points,
                &positions,
                epsilon_ms * f64::from(self.rate) / 1000.0,
            );
            let mut beats = Vec::with_capacity(fitted.len());
            for index in fitted {
                beats.push(Beat {
                    id: self.id(),
                    frame: points[index],
                    offset: offsets[index],
                });
            }
            // Refitting intentionally replaces authored and former-boundary anchors.
            self.measures[i].beats = beats;
        }
        Ok(())
    }
}
