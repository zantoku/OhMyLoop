use super::*;
use serde::Serialize;
use std::collections::HashMap;

#[derive(Clone, Debug, Serialize)]
pub struct GridPoint {
    pub id: Option<String>,
    pub frame: u64,
    pub quarter: f64,
    pub label: String,
}
#[derive(Clone, Debug, Serialize)]
pub struct MeasureView {
    pub id: String,
    pub start_id: String,
    pub end_id: String,
    pub start: u64,
    pub end: u64,
    pub number: Option<u32>,
    pub label: String,
    pub meter: Meter,
    pub kind: MeasureKind,
    pub length: Fraction,
    pub protected: bool,
    pub grid: Vec<GridPoint>,
    pub split_points: Vec<SplitPoint>,
    pub tempo_bpm: f64,
}
#[derive(Clone, Debug, Serialize)]
pub struct SplitPoint {
    pub beat: u32,
    pub frame: u64,
    pub anchor: bool,
}
#[derive(Clone, Debug, Serialize)]
pub struct MapView {
    pub revision: u64,
    pub measures: Vec<MeasureView>,
}
#[derive(Clone, Debug, Serialize)]
pub struct Position {
    pub measure: String,
    pub label: String,
    pub beat: f64,
    pub tempo_bpm: f64,
    pub pulse: Fraction,
}
#[derive(Clone, Debug, Serialize)]
pub struct DetectedDraftSummary {
    pub epsilon_ms: f64,
    pub anchor_count: usize,
    pub pulse_count: usize,
    pub pulses_per_measure: usize,
    pub full_measures: usize,
    pub partial_pulses: usize,
    pub measure_count: usize,
    pub first_frame: u64,
    pub last_frame: u64,
}
#[derive(Serialize)]
pub struct Preview {
    pub draft: Option<DetectedDraftSummary>,
    pub map: MeasureMap,
    pub view: MapView,
    pub warnings: Vec<String>,
}

impl MeasureMap {
    pub fn frame_at(&self, measure: &Measure, quarter: f64) -> u64 {
        let (start, end) = self.bounds(measure);
        Self::frame_between(measure, start, end, quarter)
    }
    pub(super) fn frame_between(measure: &Measure, start: u64, end: u64, quarter: f64) -> u64 {
        let mut a = (0.0, start);
        for b in measure
            .beats
            .iter()
            .map(|v| (v.offset.value(), v.frame))
            .chain([(measure.length.value(), end)])
        {
            if quarter <= b.0 {
                return a.1
                    + ((b.1 - a.1) as f64 * ((quarter - a.0) / (b.0 - a.0)).clamp(0.0, 1.0)).round()
                        as u64;
            }
            a = b;
        }
        end
    }
    pub fn quarter_at(&self, measure: &Measure, frame: u64) -> f64 {
        let (start, end) = self.bounds(measure);
        let mut a = (0.0, start);
        for b in measure
            .beats
            .iter()
            .map(|v| (v.offset.value(), v.frame))
            .chain([(measure.length.value(), end)])
        {
            if frame <= b.1 {
                return a.0
                    + (b.0 - a.0)
                        * (frame.saturating_sub(a.1) as f64 / (b.1 - a.1) as f64).clamp(0.0, 1.0);
            }
            a = b;
        }
        measure.length.value()
    }
    pub fn view(&self) -> MapView {
        let bounds: HashMap<_, _> = self
            .boundaries
            .iter()
            .map(|b| (b.id.as_str(), b.frame))
            .collect();
        let mut number = self.first_number;
        let measures = self
            .measures
            .iter()
            .map(|m| {
                let (start, end) = (bounds[m.start.as_str()], bounds[m.end.as_str()]);
                let pickup = m.kind == MeasureKind::Pickup;
                let label = if pickup {
                    "Pickup".into()
                } else {
                    number.to_string()
                };
                let display = (!pickup).then_some(number);
                if !pickup {
                    number += 1;
                }
                let mut grid = vec![];
                let origin = m.offset.value();
                let length = m.length.value();
                let positions: Vec<f64> = if m.meter.grouping.is_empty() {
                    let pulse = m.meter.pulse.value();
                    let count = (m.meter.length().value() / pulse).ceil() as usize;
                    // Limit visual grids independently from exact custom anchors.
                    (0..count.min(512)).map(|i| i as f64 * pulse).collect()
                } else {
                    let mut q = 0.0;
                    m.meter
                        .grouping
                        .iter()
                        .map(|n| {
                            let at = q;
                            q += f64::from(*n) * 4.0 / f64::from(m.meter.denominator);
                            at
                        })
                        .collect()
                };
                for absolute in &positions {
                    let q = absolute - origin;
                    if q >= 0.0 && q < length {
                        let anchor = m.beats.iter().find(|b| (b.offset.value() - q).abs() < 1e-9);
                        grid.push(GridPoint {
                            id: anchor.map(|b| b.id.clone()),
                            frame: Self::frame_between(m, start, end, q),
                            quarter: q,
                            label: (1.0 + absolute / m.meter.pulse.value()).to_string(),
                        });
                    }
                }
                for b in &m.beats {
                    if !grid.iter().any(|v| v.id.as_ref() == Some(&b.id)) {
                        grid.push(GridPoint {
                            id: Some(b.id.clone()),
                            frame: b.frame,
                            quarter: b.offset.value(),
                            label: "Anchor".into(),
                        });
                    }
                }
                grid.sort_by_key(|p| p.frame);
                let split_points = (0..(m.meter.length().value() / m.meter.pulse.value())
                    .ceil()
                    .min(512.0) as u32)
                    .filter_map(|index| {
                        let q = f64::from(index) * m.meter.pulse.value() - origin;
                        (q > 0.0 && q < length).then(|| SplitPoint {
                            beat: index + 1,
                            frame: Self::frame_between(m, start, end, q),
                            anchor: m.beats.iter().any(|b| (b.offset.value() - q).abs() < 1e-9),
                        })
                    })
                    .collect();
                MeasureView {
                    id: m.id.clone(),
                    start_id: m.start.clone(),
                    end_id: m.end.clone(),
                    start,
                    end,
                    number: display,
                    label,
                    meter: m.meter.clone(),
                    kind: m.kind,
                    length: m.length,
                    protected: m.protected,
                    grid,
                    split_points,
                    tempo_bpm: 60.0 * m.length.value() * f64::from(self.rate)
                        / ((end - start) as f64 * m.meter.pulse.value()),
                }
            })
            .collect();
        MapView {
            revision: self.revision,
            measures,
        }
    }
    fn tempo_at(&self, measure: &Measure, frame: u64) -> f64 {
        let (start, end) = self.bounds(measure);
        let mut left = (0.0, start);
        for right in measure
            .beats
            .iter()
            .map(|b| (b.offset.value(), b.frame))
            .chain([(measure.length.value(), end)])
        {
            if frame < right.1 {
                return 60.0 * (right.0 - left.0) * f64::from(self.rate)
                    / ((right.1 - left.1) as f64 * measure.meter.pulse.value());
            }
            left = right;
        }
        0.0
    }
    pub fn position(&self, frame: u64) -> Option<Position> {
        let bounds: HashMap<_, _> = self
            .boundaries
            .iter()
            .map(|b| (b.id.as_str(), b.frame))
            .collect();
        let mut number = self.first_number;
        for m in &self.measures {
            let (start, end) = (bounds[m.start.as_str()], bounds[m.end.as_str()]);
            let pickup = m.kind == MeasureKind::Pickup;
            if start <= frame && frame < end {
                return Some(Position {
                    measure: m.id.clone(),
                    label: if pickup {
                        "Pickup".into()
                    } else {
                        number.to_string()
                    },
                    tempo_bpm: self.tempo_at(m, frame),
                    pulse: m.meter.pulse,
                    beat: 1.0
                        + (m.offset.value() + self.quarter_at(m, frame)) / m.meter.pulse.value(),
                });
            }
            if !pickup {
                number += 1;
            }
        }
        None
    }
}
