//! Accepted musical definitions. Source frames and notated quarter notes are
//! independent of playback controls and of any automatic analysis backend.
use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

const MAX_MEASURES: usize = 10_000;
const MAX_BEATS: usize = 80_000;

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Fraction {
    pub num: u32,
    pub den: u32,
}
impl Fraction {
    pub fn new(num: u32, den: u32) -> Self {
        let mut a = num;
        let mut b = den;
        while b != 0 {
            (a, b) = (b, a % b);
        }
        let divisor = a.max(1);
        Self {
            num: num / divisor,
            den: den / divisor,
        }
    }
    pub fn value(self) -> f64 {
        self.num as f64 / self.den as f64
    }
    fn validate(self, positive: bool) -> Result<()> {
        ensure!(
            self.den > 0 && self.den <= 960 && self.num <= 65_536 && (!positive || self.num > 0),
            "Invalid notated duration"
        );
        Ok(())
    }
    fn add(self, other: Self) -> Self {
        Self::new(
            self.num * other.den + other.num * self.den,
            self.den * other.den,
        )
    }
    fn sub(self, other: Self) -> Result<Self> {
        ensure!(
            self.value() >= other.value(),
            "Notated position is outside the measure"
        );
        Ok(Self::new(
            self.num * other.den - other.num * self.den,
            self.den * other.den,
        ))
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Meter {
    pub numerator: u32,
    pub denominator: u32,
    pub pulse: Fraction,
    pub grouping: Vec<u32>,
}
impl Meter {
    pub fn common() -> Self {
        Self {
            numerator: 4,
            denominator: 4,
            pulse: Fraction::new(1, 1),
            grouping: vec![],
        }
    }
    pub fn length(&self) -> Fraction {
        Fraction::new(self.numerator * 4, self.denominator)
    }
    fn validate(&self) -> Result<()> {
        ensure!(
            (1..=99).contains(&self.numerator)
                && [1, 2, 4, 8, 16, 32, 64].contains(&self.denominator),
            "Choose a valid time signature"
        );
        self.pulse.validate(true)?;
        ensure!(
            self.pulse.value() <= self.length().value(),
            "Pulse is longer than the measure"
        );
        ensure!(
            self.grouping.len() <= 99
                && (self.grouping.is_empty()
                    || (self.grouping.iter().all(|v| *v > 0 && *v <= 99)
                        && self.grouping.iter().sum::<u32>() == self.numerator)),
            "Grouping must add up to the time-signature numerator"
        );
        Ok(())
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Boundary {
    pub id: String,
    pub frame: u64,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Beat {
    pub id: String,
    pub frame: u64,
    pub offset: Fraction,
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MeasureKind {
    Full,
    Pickup,
    Incomplete,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Measure {
    pub id: String,
    pub start: String,
    pub end: String,
    pub meter: Meter,
    pub kind: MeasureKind,
    pub length: Fraction,
    pub offset: Fraction,
    pub beats: Vec<Beat>,
    pub protected: bool,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct MeasureMap {
    pub version: u32,
    pub revision: u64,
    pub rate: u32,
    pub frames: u64,
    pub next_id: u64,
    pub first_number: u32,
    pub boundaries: Vec<Boundary>,
    pub measures: Vec<Measure>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum Edit {
    BuildFromPulses {
        pulses: Vec<u64>,
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
    Merge {
        id: String,
        meter: Meter,
    },
    Respace {
        first: String,
        last: String,
    },
}

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
}
#[derive(Serialize)]
pub struct Preview {
    pub map: MeasureMap,
    pub view: MapView,
}

impl MeasureMap {
    pub fn empty(rate: u32, frames: u64) -> Self {
        Self {
            version: 1,
            revision: 0,
            rate,
            frames,
            next_id: 1,
            first_number: 1,
            boundaries: vec![],
            measures: vec![],
        }
    }
    fn id(&mut self) -> String {
        let id = format!("map-{}", self.next_id);
        self.next_id += 1;
        id
    }
    fn boundary(&mut self, frame: u64) -> String {
        if let Some(b) = self.boundaries.iter().find(|b| b.frame == frame) {
            return b.id.clone();
        }
        let id = self.id();
        self.boundaries.push(Boundary {
            id: id.clone(),
            frame,
        });
        id
    }
    pub fn validate(&self) -> Result<()> {
        ensure!(
            self.version == 1 && self.rate > 0 && self.frames > 0 && self.frames < (1 << 53),
            "Unsupported musical map or source coordinates"
        );
        ensure!(
            self.revision < u64::MAX
                && self.next_id > 0
                && self.next_id < u64::MAX - 100_000
                && (1..=1_000_000).contains(&self.first_number),
            "Invalid map revision or numbering origin"
        );
        ensure!(
            self.measures.len() <= MAX_MEASURES
                && self.boundaries.len() <= MAX_MEASURES * 2
                && self.measures.iter().map(|m| m.beats.len()).sum::<usize>() <= MAX_BEATS,
            "Measure map is too large"
        );
        let mut ids = HashSet::new();
        let mut add_id = |id: &String| -> Result<()> {
            ensure!(
                !id.is_empty() && id.len() <= 128 && ids.insert(id.clone()),
                "Map IDs must be unique"
            );
            Ok(())
        };
        let mut frames = HashSet::new();
        let mut bounds = HashMap::new();
        for b in &self.boundaries {
            add_id(&b.id)?;
            ensure!(
                b.frame <= self.frames && frames.insert(b.frame),
                "Boundaries must have distinct positions inside the song"
            );
            bounds.insert(&b.id, b.frame);
        }
        let mut last_end = 0;
        let mut used = HashSet::new();
        for (index, m) in self.measures.iter().enumerate() {
            add_id(&m.id)?;
            m.meter.validate()?;
            m.length.validate(true)?;
            m.offset.validate(false)?;
            let start = *bounds
                .get(&m.start)
                .context("Missing measure start boundary")?;
            let end = *bounds.get(&m.end).context("Missing measure end boundary")?;
            ensure!(
                start < end && start >= last_end,
                "Measures must be ordered and cannot overlap"
            );
            last_end = end;
            used.insert(&m.start);
            used.insert(&m.end);
            let nominal = m.meter.length().value();
            ensure!(
                m.length.value() <= nominal
                    && m.offset.value() + m.length.value() <= nominal + 1e-9,
                "Notated extent exceeds the meter"
            );
            match m.kind {
                MeasureKind::Full => ensure!(
                    m.length.value() == nominal && m.offset.num == 0,
                    "A full measure must match its meter"
                ),
                MeasureKind::Pickup => ensure!(
                    index == 0
                        && m.length.value() < nominal
                        && (m.offset.value() + m.length.value() - nominal).abs() < 1e-9,
                    "Only the leading partial measure can be a pickup"
                ),
                MeasureKind::Incomplete => ensure!(
                    m.length.value() < nominal && m.offset.num == 0,
                    "An incomplete measure must be shorter than its meter"
                ),
            }
            let mut prior_frame = start;
            let mut prior_quarter = 0.0;
            for beat in &m.beats {
                add_id(&beat.id)?;
                beat.offset.validate(true)?;
                ensure!(
                    beat.frame > prior_frame
                        && beat.frame < end
                        && beat.offset.value() > prior_quarter
                        && beat.offset.value() < m.length.value(),
                    "Beat anchors must advance in source and musical time inside the measure"
                );
                prior_frame = beat.frame;
                prior_quarter = beat.offset.value();
            }
        }
        ensure!(
            used.len() == self.boundaries.len(),
            "Map has unused boundaries"
        );
        // Generated IDs must remain unique after undo and file reload.
        for id in ids {
            if let Some(n) = id.strip_prefix("map-").and_then(|n| n.parse::<u64>().ok()) {
                ensure!(n < self.next_id, "Map ID counter is stale");
            }
        }
        Ok(())
    }
    fn bounds(&self, measure: &Measure) -> (u64, u64) {
        let frame = |id: &str| self.boundaries.iter().find(|b| b.id == id).unwrap().frame;
        (frame(&measure.start), frame(&measure.end))
    }
    fn index(&self, id: &str) -> Result<usize> {
        self.measures
            .iter()
            .position(|m| m.id == id)
            .context("Select an existing measure")
    }
    fn editable(&self, index: usize) -> Result<()> {
        ensure!(
            !self.measures[index].protected,
            "Unprotect this measure before changing it"
        );
        Ok(())
    }
    fn clean(&mut self) {
        let used: HashSet<_> = self
            .measures
            .iter()
            .flat_map(|m| [&m.start, &m.end])
            .cloned()
            .collect();
        self.boundaries.retain(|b| used.contains(&b.id));
        self.boundaries.sort_by_key(|b| b.frame);
    }
    pub fn edit(&self, edit: Edit) -> Result<Preview> {
        self.validate()?;
        let mut next = self.clone();
        match edit {
            Edit::BuildFromPulses {
                pulses,
                start,
                end,
                meter,
            } => {
                meter.validate()?;
                ensure!(
                    pulses.len() <= MAX_BEATS
                        && pulses.windows(2).all(|w| w[0] < w[1])
                        && pulses.iter().all(|p| *p < self.frames),
                    "Invalid detected pulse positions"
                );
                let unit_num = meter.numerator * 4 * meter.pulse.den;
                let unit_den = meter.denominator * meter.pulse.num;
                ensure!(
                    unit_num.is_multiple_of(unit_den),
                    "This pulse does not divide the meter. Choose a smaller pulse note value."
                );
                let per_bar = (unit_num / unit_den) as usize;
                ensure!(per_bar > 0 && per_bar <= 512, "Unsupported pulse count");
                let points: Vec<_> = pulses
                    .into_iter()
                    .filter(|p| *p >= start && *p <= end)
                    .collect();
                ensure!(
                    points.len() >= 2,
                    "Choose a span containing at least two detected pulses"
                );
                let begin = points[0];
                let finish = *points.last().unwrap();
                let intervals = points.len() - 1;
                let count = intervals.div_ceil(per_bar);
                ensure!(count <= 4096, "Analyze a smaller span");
                next = self
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
                    for j in 1..remaining {
                        let id = next.id();
                        next.measures[i].beats.push(Beat {
                            id,
                            frame: points[first + j],
                            offset: Fraction::new(meter.pulse.num * j as u32, meter.pulse.den),
                        });
                    }
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
                for m in &next.measures {
                    let (a, b) = next.bounds(m);
                    if a < end && b > start {
                        ensure!(
                            a >= start && b <= end && !m.protected,
                            "Build span must contain whole unprotected measures"
                        );
                    }
                }
                let retained = next
                    .measures
                    .iter()
                    .filter(|m| {
                        let (a, b) = next.bounds(m);
                        a >= end || b <= start
                    })
                    .cloned()
                    .collect();
                next.measures = retained;
                for i in 0..count {
                    let a = start + (end - start) * u64::from(i) / u64::from(count);
                    let b = start + (end - start) * u64::from(i + 1) / u64::from(count);
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
                next.boundaries[boundary].frame = frame;
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
                    m.kind = if i == first { kind } else { MeasureKind::Full };
                    m.length = if m.kind == MeasureKind::Full {
                        meter.length()
                    } else {
                        length
                    };
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
            Edit::Split { id, frame, left } => {
                left.validate(true)?;
                let i = next.index(&id)?;
                next.editable(i)?;
                let old = next.measures[i].clone();
                let (a, b) = next.bounds(&old);
                ensure!(
                    frame > a
                        && frame < b
                        && left.value() < old.length.value()
                        && old.kind != MeasureKind::Pickup,
                    "Split inside a non-pickup measure with a valid notated duration"
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
                first
                    .beats
                    .retain(|beat| beat.frame < frame && beat.offset.value() < left.value());
                let mut second = old;
                second.id = new_id;
                second.start = boundary;
                second.length = right;
                second.kind = MeasureKind::Incomplete;
                second
                    .beats
                    .retain(|beat| beat.frame > frame && beat.offset.value() > left.value());
                for beat in &mut second.beats {
                    beat.offset = beat.offset.sub(left)?;
                }
                next.measures.splice(i..=i, [first, second]);
            }
            Edit::Merge { id, meter } => {
                meter.validate()?;
                let i = next.index(&id)?;
                next.editable(i)?;
                ensure!(i + 1 < next.measures.len(), "No following measure to merge");
                next.editable(i + 1)?;
                let mut first = next.measures[i].clone();
                let second = next.measures[i + 1].clone();
                ensure!(
                    first.end == second.start && first.kind != MeasureKind::Pickup,
                    "Merge adjacent non-pickup measures"
                );
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
                first.kind = if first.length.value() == first.meter.length().value() {
                    MeasureKind::Full
                } else {
                    MeasureKind::Incomplete
                };
                next.measures.splice(i..=i + 1, [first]);
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
        Ok(Preview { map: next, view })
    }
    pub fn frame_at(&self, measure: &Measure, quarter: f64) -> u64 {
        let (start, end) = self.bounds(measure);
        Self::frame_between(measure, start, end, quarter)
    }
    fn frame_between(measure: &Measure, start: u64, end: u64, quarter: f64) -> u64 {
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
                }
            })
            .collect();
        MapView {
            revision: self.revision,
            measures,
        }
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
