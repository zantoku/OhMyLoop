use super::fitting::{default_beat_epsilon_ms, fitted_anchor_indices};
use super::*;
use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum Edit {
    Clear,
    SetOrigin {
        first_number: u32,
    },
    BuildFromPulses {
        pulses: Vec<u64>,
        #[serde(default)]
        downbeats: Vec<u64>,
        #[serde(default)]
        first_downbeat: Option<u64>,
        #[serde(default = "default_beat_epsilon_ms")]
        epsilon_ms: f64,
        start: u64,
        end: u64,
        meter: Meter,
    },
    Build {
        start: u64,
        end: u64,
        count: u32,
        meter: Meter,
    },
    MoveBoundary {
        id: String,
        frame: u64,
    },
    SetNotation {
        id: String,
        meter: Meter,
        kind: MeasureKind,
        length: Fraction,
        following: bool,
    },
    SetProtected {
        id: String,
        protected: bool,
    },
    Remove {
        id: String,
    },
    SetBeat {
        measure: String,
        id: Option<String>,
        frame: u64,
        offset: Fraction,
    },
    RemoveBeat {
        measure: String,
        id: String,
    },
    Split {
        id: String,
        frame: u64,
        left: Fraction,
    },
    SplitAtBeat {
        id: String,
        beat: u32,
    },
    Merge {
        id: String,
    },
    Reclaim {
        start: u64,
        end: u64,
        meter: Meter,
    },
    Respace {
        first: String,
        last: String,
    },
}

impl MeasureMap {
    pub(super) fn bounds(&self, measure: &Measure) -> (u64, u64) {
        let frame = |id: &str| self.boundaries.iter().find(|b| b.id == id).unwrap().frame;
        (frame(&measure.start), frame(&measure.end))
    }
    pub(super) fn index(&self, id: &str) -> Result<usize> {
        self.measures
            .iter()
            .position(|m| m.id == id)
            .context("Select an existing measure")
    }
    pub(super) fn editable(&self, index: usize) -> Result<()> {
        ensure!(
            !self.measures[index].protected,
            "Unprotect this measure before changing it"
        );
        Ok(())
    }
    pub(super) fn clean(&mut self) {
        let used: HashSet<_> = self
            .measures
            .iter()
            .flat_map(|m| [&m.start, &m.end])
            .cloned()
            .collect();
        self.boundaries.retain(|b| used.contains(&b.id));
        self.boundaries.sort_by_key(|b| b.frame);
    }
    fn remove_span(&mut self, start: u64, end: u64) -> Result<()> {
        for m in &self.measures {
            let (a, b) = self.bounds(m);
            if a < end && b > start {
                ensure!(
                    a >= start && b <= end && !m.protected,
                    "Build span must contain whole unprotected measures"
                );
            }
        }
        self.measures = self
            .measures
            .iter()
            .filter(|m| {
                let (a, b) = self.bounds(m);
                a >= end || b <= start
            })
            .cloned()
            .collect();
        Ok(())
    }
    pub fn edit(&self, edit: Edit) -> Result<Preview> {
        self.validate()?;
        let mut next = self.clone();
        let mut warnings = Vec::new();
        let mut draft = None;
        match edit {
            Edit::Clear => {
                next.measures.clear();
                next.boundaries.clear();
                next.first_number = 1;
            }
            Edit::SetOrigin { first_number } => {
                next.first_number = first_number;
            }
            Edit::BuildFromPulses {
                pulses,
                downbeats,
                first_downbeat,
                epsilon_ms,
                start,
                end,
                meter,
            } => {
                meter.validate()?;
                ensure!(
                    epsilon_ms.is_finite() && (0.0..=1000.0).contains(&epsilon_ms),
                    "Beat timing tolerance must be between 0 and 1000 ms"
                );
                ensure!(
                    pulses.len() <= MAX_BEATS
                        && pulses.windows(2).all(|w| w[0] < w[1])
                        && pulses.iter().all(|p| *p < self.frames),
                    "Invalid detected pulse positions"
                );
                ensure!(
                    start < end && end <= self.frames,
                    "Choose a span inside the song"
                );
                ensure!(
                    downbeats.len() <= MAX_BEATS
                        && downbeats.windows(2).all(|w| w[0] < w[1])
                        && downbeats.iter().all(|p| *p < self.frames),
                    "Invalid downbeat suggestions"
                );
                let unit_num = meter.numerator * 4 * meter.pulse.den;
                let unit_den = meter.denominator * meter.pulse.num;
                ensure!(
                    unit_num.is_multiple_of(unit_den),
                    "This pulse does not divide the meter. Choose a smaller pulse note value."
                );
                let per_bar = (unit_num / unit_den) as usize;
                ensure!(per_bar > 0 && per_bar <= 512, "Unsupported pulse count");
                let mut points: Vec<_> = pulses
                    .into_iter()
                    .filter(|p| *p >= start && *p <= end)
                    .collect();
                ensure!(
                    points.len() >= 2,
                    "Choose a span containing at least two detected pulses"
                );
                if let Some(requested) = first_downbeat {
                    ensure!(
                        requested >= start && requested <= end,
                        "Choose a first downbeat inside the selected span"
                    );
                    let first = *points.iter().min_by_key(|p| p.abs_diff(requested)).unwrap();
                    points.retain(|p| *p >= first);
                    ensure!(
                        points.len() >= 2,
                        "Choose a first downbeat with at least one following detected pulse"
                    );
                }
                let begin = points[0];
                let finish = *points.last().unwrap();
                let intervals = points.len() - 1;
                let count = intervals.div_ceil(per_bar);
                ensure!(count <= 4096, "Analyze a smaller span");
                draft = Some(DetectedDraftSummary {
                    pulse_count: points.len(),
                    pulses_per_measure: per_bar,
                    full_measures: intervals / per_bar,
                    partial_pulses: intervals % per_bar,
                    measure_count: count,
                    first_frame: begin,
                    last_frame: finish,
                    epsilon_ms,
                    anchor_count: 0,
                });
                // Replace the user-selected whole-measure span, then leave any
                // unobserved leading/trailing audio unmapped. Detected edges need
                // not coincide with the edges of an older draft.
                next.remove_span(start, end)?;
                next.clean();
                next = next
                    .edit(Edit::Build {
                        start: begin,
                        end: finish,
                        count: count as u32,
                        meter: meter.clone(),
                    })?
                    .map;
                next.revision = self.revision;
                let indices: Vec<_> = next
                    .measures
                    .iter()
                    .enumerate()
                    .filter(|(_, m)| {
                        let (a, b) = next.bounds(m);
                        a >= begin && b <= finish
                    })
                    .map(|(i, _)| i)
                    .collect();
                for (bar, i) in indices.into_iter().enumerate() {
                    let first = bar * per_bar;
                    let remaining = (intervals - first).min(per_bar);
                    let a = next.measures[i].start.clone();
                    let b = next.measures[i].end.clone();
                    next.boundaries
                        .iter_mut()
                        .find(|v| v.id == a)
                        .unwrap()
                        .frame = points[first];
                    next.boundaries
                        .iter_mut()
                        .find(|v| v.id == b)
                        .unwrap()
                        .frame = points[first + remaining];
                    let length = Fraction::new(meter.pulse.num * remaining as u32, meter.pulse.den);
                    next.measures[i].length = length;
                    next.measures[i].kind = if remaining == per_bar {
                        MeasureKind::Full
                    } else {
                        MeasureKind::Incomplete
                    };
                    let anchors = fitted_anchor_indices(
                        &points[first..=first + remaining],
                        epsilon_ms * f64::from(self.rate) / 1000.0,
                    );
                    draft.as_mut().unwrap().anchor_count += anchors.len();
                    for j in anchors {
                        let id = next.id();
                        next.measures[i].beats.push(Beat {
                            id,
                            frame: points[first + j],
                            offset: Fraction::new(meter.pulse.num * j as u32, meter.pulse.den),
                        });
                    }
                }
                let starts: HashSet<_> = next
                    .measures
                    .iter()
                    .filter_map(|m| {
                        let frame = next.bounds(m).0;
                        (frame >= begin && frame < finish).then_some(frame)
                    })
                    .collect();
                let detected: HashSet<_> = downbeats
                    .into_iter()
                    .filter(|p| *p >= begin && *p < finish)
                    .collect();
                let conflicts = starts.symmetric_difference(&detected).count();
                if detected.is_empty() {
                    warnings.push("No downbeats support this span. Confirm the first full downbeat and meter by listening.".into());
                } else if conflicts > 0 {
                    warnings.push(format!("{conflicts} measure starts and suggested downbeats disagree. Check the first downbeat, pulse note value, or missing/extra beats."));
                }
            }
            Edit::Build {
                start,
                end,
                count,
                meter,
            } => {
                meter.validate()?;
                ensure!(
                    start < end
                        && end <= self.frames
                        && count > 0
                        && count <= 4096
                        && end - start >= u64::from(count),
                    "Choose a span and a valid number of measures"
                );
                next.remove_span(start, end)?;

                for i in 0..count {
                    let a = start
                        + ((u128::from(end - start) * u128::from(i)) / u128::from(count)) as u64;
                    let b = start
                        + ((u128::from(end - start) * u128::from(i + 1)) / u128::from(count))
                            as u64;
                    let start = next.boundary(a);
                    let end = next.boundary(b);
                    let id = next.id();
                    next.measures.push(Measure {
                        id,
                        start,
                        end,
                        meter: meter.clone(),
                        kind: MeasureKind::Full,
                        length: meter.length(),
                        offset: Fraction::new(0, 1),
                        beats: vec![],
                        protected: false,
                    });
                }
                let positions: HashMap<_, _> = next
                    .boundaries
                    .iter()
                    .map(|b| (b.id.clone(), b.frame))
                    .collect();
                next.measures.sort_by_key(|m| positions[&m.start]);
            }
            Edit::MoveBoundary { id, frame } => {
                let boundary = next
                    .boundaries
                    .iter()
                    .position(|b| b.id == id)
                    .context("Select an existing boundary")?;
                for (i, m) in next.measures.iter().enumerate() {
                    if m.start == id || m.end == id {
                        next.editable(i)?;
                    }
                }
                if next.boundaries[boundary].frame != frame {
                    next.boundaries[boundary].frame = frame;
                    // Resizing authors a uniform tempo within each affected
                    // measure. Keep outer bounds and notation; discard the old
                    // piecewise timing instead of leaving anchors pinned in time.
                    for m in &mut next.measures {
                        if m.start == id || m.end == id {
                            m.beats.clear();
                        }
                    }
                }
            }
            Edit::SetNotation {
                id,
                meter,
                kind,
                length,
                following,
            } => {
                meter.validate()?;
                length.validate(true)?;
                let first = next.index(&id)?;
                let original = next.measures[first].meter.clone();
                let end = if following {
                    (first + 1..next.measures.len())
                        .find(|i| {
                            next.measures[*i].meter != original
                                || next.bounds(&next.measures[*i - 1]).1
                                    != next.bounds(&next.measures[*i]).0
                        })
                        .unwrap_or(next.measures.len())
                } else {
                    first + 1
                };
                for i in first..end {
                    next.editable(i)?;
                    let m = &mut next.measures[i];
                    m.meter = meter.clone();
                    if i == first {
                        m.kind = kind;
                        m.length = length;
                    }
                    if m.kind == MeasureKind::Full {
                        m.length = meter.length();
                    }
                    // Following meter changes preserve explicit partial extents;
                    // validation rejects any that no longer fit the new meter.
                    m.offset = if m.kind == MeasureKind::Pickup {
                        meter.length().sub(m.length)?
                    } else {
                        Fraction::new(0, 1)
                    };
                    // This explicit regrid command resets within-measure interpolation.
                    m.beats.clear();
                }
            }
            Edit::SetProtected { id, protected } => {
                let i = next.index(&id)?;
                next.measures[i].protected = protected;
            }
            Edit::Remove { id } => {
                let i = next.index(&id)?;
                next.editable(i)?;
                next.measures.remove(i);
            }
            Edit::SetBeat {
                measure,
                id,
                frame,
                offset,
            } => {
                let i = next.index(&measure)?;
                next.editable(i)?;
                offset.validate(true)?;
                let beat_id = if let Some(id) = id {
                    ensure!(
                        next.measures[i].beats.iter().any(|b| b.id == id),
                        "Select an existing beat anchor"
                    );
                    id
                } else {
                    next.id()
                };
                next.measures[i].beats.retain(|b| b.id != beat_id);
                next.measures[i].beats.push(Beat {
                    id: beat_id,
                    frame,
                    offset,
                });
                next.measures[i].beats.sort_by_key(|b| b.frame);
            }
            Edit::RemoveBeat { measure, id } => {
                let i = next.index(&measure)?;
                next.editable(i)?;
                ensure!(
                    next.measures[i].beats.iter().any(|b| b.id == id),
                    "Select an existing beat anchor"
                );
                next.measures[i].beats.retain(|b| b.id != id);
            }
            Edit::SplitAtBeat { id, beat } => {
                ensure!(
                    (1..=512).contains(&beat),
                    "Select an internal beat to split at"
                );
                let i = next.index(&id)?;
                let measure = &next.measures[i];
                let left = Fraction::new(
                    measure.meter.pulse.num * (beat - 1),
                    measure.meter.pulse.den,
                )
                .sub(measure.offset)?;
                let frame = next.frame_at(measure, left.value());
                next = next.edit(Edit::Split { id, frame, left })?.map;
                next.revision = self.revision;
            }
            Edit::Split { id, frame, left } => {
                left.validate(true)?;
                let i = next.index(&id)?;
                next.editable(i)?;
                let old = next.measures[i].clone();
                let (a, b) = next.bounds(&old);
                ensure!(
                    frame > a && frame < b && left.value() < old.length.value(),
                    "Split at an internal beat with a valid notated duration"
                );
                for beat in &old.beats {
                    ensure!(
                        (beat.frame < frame && beat.offset.value() < left.value())
                            || (beat.frame > frame && beat.offset.value() > left.value())
                            || (beat.frame == frame && beat.offset.value() == left.value()),
                        "Split would discard a conflicting beat anchor; correct its musical position first"
                    );
                }
                let boundary = next.boundary(frame);
                let new_id = next.id();
                let right = old.length.sub(left)?;
                let mut first = old.clone();
                first.end = boundary.clone();
                first.length = left;
                first.kind = MeasureKind::Incomplete;
                first.offset = Fraction::new(0, 1);
                first
                    .beats
                    .retain(|beat| beat.frame < frame && beat.offset.value() < left.value());
                let mut second = old;
                second.id = new_id;
                second.start = boundary;
                second.length = right;
                second.kind = MeasureKind::Incomplete;
                second.offset = Fraction::new(0, 1);
                second
                    .beats
                    .retain(|beat| beat.frame > frame && beat.offset.value() > left.value());
                for beat in &mut second.beats {
                    beat.offset = beat.offset.sub(left)?;
                }
                next.measures.splice(i..=i, [first, second]);
            }
            Edit::Merge { id } => {
                let i = next.index(&id)?;
                next.editable(i)?;
                ensure!(i + 1 < next.measures.len(), "No following measure to merge");
                next.editable(i + 1)?;
                let mut first = next.measures[i].clone();
                let second = next.measures[i + 1].clone();
                ensure!(
                    first.end == second.start,
                    "Join adjacent measures without free time between them"
                );
                ensure!(
                    first.meter.denominator == second.meter.denominator,
                    "Joined measures must have the same meter denominator"
                );
                let mut meter = first.meter.clone();
                meter.numerator += second.meter.numerator;
                if !first.meter.grouping.is_empty() || !second.meter.grouping.is_empty() {
                    meter.grouping = if first.meter.grouping.is_empty() {
                        vec![first.meter.numerator]
                    } else {
                        first.meter.grouping.clone()
                    };
                    meter.grouping.extend(if second.meter.grouping.is_empty() {
                        vec![second.meter.numerator]
                    } else {
                        second.meter.grouping.clone()
                    });
                }
                meter.validate()?;
                let frame = next.bounds(&second).0;
                let old_length = first.length;
                let beat_id = next.id();
                first.beats.push(Beat {
                    id: beat_id,
                    frame,
                    offset: old_length,
                });
                for mut beat in second.beats {
                    beat.offset = old_length.add(beat.offset);
                    first.beats.push(beat);
                }
                first.end = second.end;
                first.length = old_length.add(second.length);
                first.meter = meter;
                first.offset = Fraction::new(0, 1);
                first.kind = if first.length.value() == first.meter.length().value() {
                    MeasureKind::Full
                } else {
                    MeasureKind::Incomplete
                };
                next.measures.splice(i..=i + 1, [first]);
            }
            Edit::Reclaim { start, end, meter } => {
                ensure!(
                    start < end && end <= self.frames,
                    "Choose an uncounted range inside the song"
                );
                ensure!(
                    next.measures.iter().all(|m| {
                        let (a, b) = next.bounds(m);
                        a >= end || b <= start
                    }),
                    "Create a measure only in uncounted audio; existing measures are unchanged"
                );
                next = next
                    .edit(Edit::Build {
                        start,
                        end,
                        count: 1,
                        meter,
                    })?
                    .map;
                next.revision = self.revision;
            }
            Edit::Respace { first, last } => {
                let a = next.index(&first)?;
                let b = next.index(&last)?;
                ensure!(a <= b, "Choose an ordered measure range");
                let start = next.bounds(&next.measures[a]).0;
                let end = next.bounds(&next.measures[b]).1;
                let mut total = 0.0;
                for i in a..=b {
                    next.editable(i)?;
                    if i > a {
                        ensure!(
                            next.measures[i - 1].end == next.measures[i].start,
                            "Cannot respace across free time"
                        );
                    }
                    total += next.measures[i].length.value();
                }
                let mut elapsed = 0.0;
                for i in a..=b {
                    let id = next.measures[i].start.clone();
                    next.boundaries
                        .iter_mut()
                        .find(|v| v.id == id)
                        .unwrap()
                        .frame = start + ((end - start) as f64 * elapsed / total).round() as u64;
                    elapsed += next.measures[i].length.value();
                    next.measures[i].beats.clear();
                }
            }
        }
        next.clean();
        next.revision += 1;
        next.validate()?;
        let view = next.view();
        Ok(Preview {
            map: next,
            view,
            warnings,
            draft,
        })
    }
}
