use super::*;
use anyhow::{Context, Result, ensure};
use std::collections::{HashMap, HashSet};

impl MeasureMap {
    pub fn validate(&self) -> Result<()> {
        ensure!(
            self.version == 1 && self.rate > 0 && self.frames > 0 && self.frames < (1 << 53),
            "Unsupported musical map or source coordinates"
        );
        ensure!(
            self.revision < MAX_SAFE_INTEGER
                && self.next_id > 0
                && self.next_id < MAX_SAFE_INTEGER - 100_000
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
}
