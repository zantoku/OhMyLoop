//! Repartition a counted suffix without moving the authored timing curve.
use super::*;
use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};
use std::{cmp::Ordering, collections::HashMap};

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ShiftDirection {
    Left,
    Right,
}

// Exact cumulative quarters can exceed the storage limits for local offsets.
// Checked arithmetic rejects incompatible custom fractions rather than panicking.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Quarter {
    num: u64,
    den: u64,
}
impl Quarter {
    fn new(num: u128, den: u128) -> Result<Self> {
        let (mut a, mut b) = (num, den);
        while b != 0 {
            (a, b) = (b, a % b);
        }
        let divisor = a.max(1);
        Ok(Self {
            num: (num / divisor)
                .try_into()
                .context("Shift exceeds supported musical precision")?,
            den: (den / divisor)
                .try_into()
                .context("Shift exceeds supported musical precision")?,
        })
    }
    fn add(self, other: Self) -> Result<Self> {
        let num = u128::from(self.num) * u128::from(other.den);
        let extra = u128::from(other.num) * u128::from(self.den);
        Self::new(
            num.checked_add(extra)
                .context("Shift exceeds supported musical precision")?,
            u128::from(self.den) * u128::from(other.den),
        )
    }
    fn sub(self, other: Self) -> Result<Self> {
        ensure!(
            self >= other,
            "Shift would move a boundary outside the counted range; choose fewer pulses"
        );
        Self::new(
            u128::from(self.num) * u128::from(other.den)
                - u128::from(other.num) * u128::from(self.den),
            u128::from(self.den) * u128::from(other.den),
        )
    }
    fn fraction(self) -> Result<Fraction> {
        ensure!(
            self.num <= 65536 && self.den <= 960,
            "Shift creates unsupported custom beat fractions; choose a compatible pulse count"
        );
        Ok(Fraction::new(self.num as u32, self.den as u32))
    }
    fn value(self) -> f64 {
        self.num as f64 / self.den as f64
    }
}
impl From<Fraction> for Quarter {
    fn from(f: Fraction) -> Self {
        let f = Fraction::new(f.num, f.den);
        Self {
            num: u64::from(f.num),
            den: u64::from(f.den),
        }
    }
}
impl Ord for Quarter {
    fn cmp(&self, other: &Self) -> Ordering {
        (u128::from(self.num) * u128::from(other.den))
            .cmp(&(u128::from(other.num) * u128::from(self.den)))
    }
}
impl PartialOrd for Quarter {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

struct Point {
    quarter: Quarter,
    frame: u64,
    anchor: Option<String>,
}
struct Curve {
    points: Vec<Point>,
}
impl Curve {
    fn frame_at(&self, quarter: Quarter) -> Result<u64> {
        let index = self.points.partition_point(|p| p.quarter < quarter);
        let right = self
            .points
            .get(index)
            .context("Shift would pass the last mapped beat; choose fewer pulses")?;
        if right.quarter == quarter {
            return Ok(right.frame);
        }
        let left = index
            .checked_sub(1)
            .and_then(|i| self.points.get(i))
            .context("Shift would precede the counted range")?;
        let ratio = quarter.sub(left.quarter)?.value() / right.quarter.sub(left.quarter)?.value();
        Ok(left.frame + ((right.frame - left.frame) as f64 * ratio).round() as u64)
    }
}

fn extent(measure: &mut Measure, length: Fraction) -> Result<()> {
    let old_numerator = measure.meter.numerator;
    if length.value() > measure.meter.length().value() {
        let numerator = (u64::from(length.num) * u64::from(measure.meter.denominator))
            .div_ceil(u64::from(length.den) * 4);
        ensure!(
            numerator <= 99,
            "Shift would require a meter numerator above 99; choose fewer pulses"
        );
        measure.meter.numerator = numerator as u32;
        if !measure.meter.grouping.is_empty() {
            measure
                .meter
                .grouping
                .push(measure.meter.numerator - old_numerator);
        }
    }
    measure.length = length;
    let nominal = measure.meter.length();
    if length == nominal {
        measure.kind = MeasureKind::Full;
        measure.offset = Fraction::new(0, 1);
    } else if measure.kind == MeasureKind::Pickup {
        measure.offset = nominal.sub(length)?;
    } else {
        measure.kind = MeasureKind::Incomplete;
        measure.offset = Fraction::new(0, 1);
    }
    Ok(())
}

fn collinear(left: (Fraction, u64), mid: (Fraction, u64), right: (Fraction, u64)) -> Result<bool> {
    let a = Quarter::from(mid.0).sub(left.0.into())?.fraction()?;
    let b = Quarter::from(right.0).sub(mid.0.into())?.fraction()?;
    Ok(
        u128::from(mid.1 - left.1) * u128::from(b.num) * u128::from(a.den)
            == u128::from(right.1 - mid.1) * u128::from(a.num) * u128::from(b.den),
    )
}

impl MeasureMap {
    pub(super) fn shift_suffix(
        &mut self,
        id: &str,
        pulses: u32,
        direction: ShiftDirection,
    ) -> Result<()> {
        ensure!(
            (1..=64).contains(&pulses),
            "Shift by a whole number from 1 through 64 pulses"
        );
        let selected = self.index(id)?;
        ensure!(
            selected > 0,
            "Select a measure with a preceding measure to shift"
        );
        let amount = Quarter::new(
            u128::from(self.measures[selected].meter.pulse.num) * u128::from(pulses),
            u128::from(self.measures[selected].meter.pulse.den),
        )?;
        let mut first = selected - 1;
        if matches!(direction, ShiftDirection::Left) {
            // Extend the affected range backwards when more than one preceding
            // measure is consumed. Never jump over an uncounted gap.
            let mut preceding = Quarter::from(self.measures[first].length);
            while preceding < amount && first > 0 {
                ensure!(
                    self.measures[first - 1].end == self.measures[first].start,
                    "Cannot shift through free time; reclaim or map the intervening range first"
                );
                first -= 1;
                preceding = preceding.add(self.measures[first].length.into())?;
            }
        }
        for i in first..self.measures.len() {
            self.editable(i)?;
            if i > first {
                ensure!(
                    self.measures[i - 1].end == self.measures[i].start,
                    "Cannot shift through free time; reclaim or map the intervening range first"
                );
            }
        }

        let bounds: HashMap<_, _> = self
            .boundaries
            .iter()
            .map(|b| (b.id.as_str(), b.frame))
            .collect();
        let mut quarter = Quarter::from(Fraction::new(0, 1));
        let mut cuts = vec![quarter];
        let mut points = vec![Point {
            quarter,
            frame: bounds[self.measures[first].start.as_str()],
            anchor: None,
        }];
        for m in &self.measures[first..] {
            for beat in &m.beats {
                points.push(Point {
                    quarter: quarter.add(beat.offset.into())?,
                    frame: beat.frame,
                    anchor: Some(beat.id.clone()),
                });
            }
            quarter = quarter.add(m.length.into())?;
            cuts.push(quarter);
            points.push(Point {
                quarter,
                frame: bounds[m.end.as_str()],
                anchor: None,
            });
        }
        let curve = Curve { points };
        let total = *cuts.last().unwrap();
        let zero = Quarter::from(Fraction::new(0, 1));
        for cut in cuts
            .iter_mut()
            .skip(1)
            .take(self.measures.len() - first - 1)
        {
            *cut = match direction {
                ShiftDirection::Right => cut.add(amount)?.min(total),
                ShiftDirection::Left if *cut <= amount => zero,
                ShiftDirection::Left => cut.sub(amount)?,
            };
        }
        // Cuts that reach/pass a fixed endpoint are absorbed there. The
        // zero-extent measures between them disappear; audio and curve remain.
        let retained = (0..cuts.len() - 1)
            .filter(|&i| cuts[i] < cuts[i + 1])
            .collect::<Vec<_>>();
        let first_boundary = self.measures[first].start.clone();
        let last_boundary = self.measures.last().unwrap().end.clone();
        let boundary_indices: HashMap<_, _> = self
            .boundaries
            .iter()
            .enumerate()
            .map(|(i, b)| (b.id.clone(), i))
            .collect();
        let mut shifted = Vec::with_capacity(retained.len());
        for (position, &index) in retained.iter().enumerate() {
            let (start, end) = (cuts[index], cuts[index + 1]);
            let length = end.sub(start)?.fraction()?;
            let start_frame = curve.frame_at(start)?;
            let end_frame = curve.frame_at(end)?;
            ensure!(
                start_frame < end_frame,
                "Shift leaves a measure with no audio frames; choose fewer pulses"
            );
            let mut measure = self.measures[first + index].clone();
            if position == 0 || position + 1 == retained.len() {
                extent(&mut measure, length)?;
            }
            if position == 0 {
                measure.start = first_boundary.clone();
            }
            if position + 1 == retained.len() {
                measure.end = last_boundary.clone();
            }
            self.boundaries[boundary_indices[&measure.start]].frame = start_frame;
            self.boundaries[boundary_indices[&measure.end]].frame = end_frame;
            let lo = curve.points.partition_point(|p| p.quarter <= start);
            let hi = curve.points.partition_point(|p| p.quarter < end);
            let inside = &curve.points[lo..hi];
            let mut anchors = vec![];
            let mut previous = (Fraction::new(0, 1), start_frame);
            for (j, p) in inside.iter().enumerate() {
                let offset = p.quarter.sub(start)?.fraction()?;
                let next = if let Some(p) = inside.get(j + 1) {
                    (p.quarter.sub(start)?.fraction()?, p.frame)
                } else {
                    (length, end_frame)
                };
                // Preserve authored anchors. A former bar line only needs a new
                // anchor when it marks a real change of slope in source timing.
                if p.anchor.is_none() && collinear(previous, (offset, p.frame), next)? {
                    continue;
                }
                let id = p.anchor.clone().unwrap_or_else(|| self.id());
                anchors.push(Beat {
                    id,
                    frame: p.frame,
                    offset,
                });
                previous = (offset, p.frame);
            }
            measure.beats = anchors;
            shifted.push(measure);
        }
        self.measures.splice(first.., shifted);
        Ok(())
    }
}
