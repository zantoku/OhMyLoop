//! One atomic song envelope with a single recording and authored timeline.
use super::*;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AudioAsset {
    pub id: AudioAssetId,
    pub path: String,
    pub digest: String,
    pub rate: u32,
    pub frames: u64,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TimelineRecord {
    pub version: u32,
    pub id: TimelineId,
    pub rate: u32,
    pub frames: u64,
    pub sections: Vec<Section>,
    pub selected_section: Option<String>,
    pub loop_selection: Option<LoopRegion>,
    pub loop_enabled: bool,
    pub last_position: f64,
    pub measure_map: Option<crate::timing::MeasureMap>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SongRecord {
    pub version: u32,
    pub id: SongId,
    pub reference_revision: u64,
    pub asset: AudioAsset,
    pub timeline: TimelineRecord,
    pub controls: Controls,
    pub notes: String,
}
impl SongRecord {
    pub fn new(path: String, digest: String, audio: &Audio) -> Self {
        Self {
            version: 2,
            id: SongId::new(),
            reference_revision: 1,
            asset: AudioAsset {
                id: AudioAssetId::new(),
                path,
                digest,
                rate: audio.rate,
                frames: audio.frames.len() as u64,
            },
            timeline: TimelineRecord::new(audio),
            controls: Controls::default(),
            notes: String::new(),
        }
    }
    pub(super) fn read(value: serde_json::Value) -> Result<Self> {
        match value.get("version").and_then(|version| version.as_u64()) {
            Some(1) => {
                serde_json::from_value::<super::legacy::LegacySongRecord>(value)?.into_current()
            }
            Some(2) => Ok(serde_json::from_value(value)?),
            _ => anyhow::bail!("Unsupported song schema; original file preserved"),
        }
    }
    pub fn reference(&self, session: SessionId) -> Reference {
        Reference {
            song: self.id,
            asset: self.asset.id,
            timeline: self.timeline.id,
            revision: self.reference_revision,
            session,
        }
    }
    pub fn document(&self, session: SessionId) -> Result<Document> {
        self.validate()?;
        let t = &self.timeline;
        Ok(Document {
            version: 5,
            song_key: self.id.to_string(),
            reference_token: self.reference(session).token(),
            controls: self.controls,
            notes: self.notes.clone(),
            sections: t.sections.clone(),
            selected_section: t.selected_section.clone(),
            loop_selection: t.loop_selection,
            loop_enabled: t.loop_enabled,
            last_position: t.last_position,
            measure_map: t.measure_map.clone(),
        })
    }
    pub fn update_document(&mut self, d: &Document) -> Result<()> {
        let r = Reference::parse(&d.reference_token)?;
        ensure!(
            r.song == self.id
                && r.asset == self.asset.id
                && r.timeline == self.timeline.id
                && r.revision == self.reference_revision,
            "Recording changed; discard stale practice update"
        );
        d.validate(
            &self.id.to_string(),
            self.timeline.frames as f64 / self.timeline.rate as f64,
        )?;
        ensure!(
            d.measure_map.as_ref().is_none_or(
                |map| map.rate == self.timeline.rate && map.frames == self.timeline.frames
            ),
            "Map geometry does not match timeline"
        );
        self.controls = d.controls;
        self.notes = d.notes.clone();
        self.timeline.sections = d.sections.clone();
        self.timeline.selected_section = d.selected_section.clone();
        self.timeline.loop_selection = d.loop_selection;
        self.timeline.loop_enabled = d.loop_enabled;
        self.timeline.last_position = d.last_position;
        self.timeline.measure_map = d.measure_map.clone();
        Ok(())
    }
    pub fn replace(
        &mut self,
        path: String,
        digest: String,
        audio: &Audio,
        timing_unchanged: bool,
    ) -> Result<()> {
        ensure!(
            !timing_unchanged
                || (self.timeline.rate == audio.rate
                    && self.timeline.frames == audio.frames.len() as u64),
            "Unchanged timing requires the same decoded sample rate and frame count; choose changed timing"
        );
        let replacement = Self {
            version: 2,
            id: self.id,
            reference_revision: self
                .reference_revision
                .checked_add(1)
                .context("Reference revision exhausted")?,
            asset: AudioAsset {
                id: AudioAssetId::new(),
                path,
                digest,
                rate: audio.rate,
                frames: audio.frames.len() as u64,
            },
            timeline: if timing_unchanged {
                self.timeline.clone()
            } else {
                TimelineRecord::new(audio)
            },
            controls: self.controls,
            notes: self.notes.clone(),
        };
        replacement.validate()?;
        *self = replacement;
        Ok(())
    }
    pub fn validate(&self) -> Result<()> {
        ensure!(
            self.version == 2 && self.reference_revision > 0,
            "Unsupported song schema; original file preserved"
        );
        self.asset.validate()?;
        self.timeline
            .validate(self.id, self.controls, &self.notes)?;
        ensure!(
            self.asset.rate == self.timeline.rate && self.asset.frames == self.timeline.frames,
            "Recording geometry differs from timeline"
        );
        Ok(())
    }
}
impl AudioAsset {
    pub(super) fn validate(&self) -> Result<()> {
        ensure!(
            !self.path.is_empty()
                && self.path.len() <= 16384
                && !self.path.contains('\0')
                && self.digest.len() == 64
                && self.digest.bytes().all(|b| b.is_ascii_hexdigit())
                && self.rate > 0
                && self.frames > 0,
            "Invalid audio asset"
        );
        Ok(())
    }
}
impl TimelineRecord {
    fn new(audio: &Audio) -> Self {
        Self {
            version: 1,
            id: TimelineId::new(),
            rate: audio.rate,
            frames: audio.frames.len() as u64,
            sections: vec![],
            selected_section: None,
            loop_selection: None,
            loop_enabled: false,
            last_position: 0.0,
            measure_map: None,
        }
    }
    pub(super) fn validate(&self, song: SongId, controls: Controls, notes: &str) -> Result<()> {
        ensure!(
            self.version == 1 && self.rate > 0 && self.frames > 0,
            "Invalid timeline"
        );
        let mut d = Document::new(song.to_string());
        d.controls = controls;
        d.notes = notes.to_owned();
        d.sections = self.sections.clone();
        d.selected_section = self.selected_section.clone();
        d.loop_selection = self.loop_selection;
        d.loop_enabled = self.loop_enabled;
        d.last_position = self.last_position;
        d.measure_map = self.measure_map.clone();
        d.validate(&song.to_string(), self.frames as f64 / self.rate as f64)?;
        ensure!(
            self.measure_map
                .as_ref()
                .is_none_or(|map| map.rate == self.rate && map.frames == self.frames),
            "Map geometry does not match timeline"
        );
        Ok(())
    }
}
