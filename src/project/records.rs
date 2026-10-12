//! One atomic song envelope, with distinct versioned song and timeline domains.
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
pub struct Recording {
    pub asset: AudioAssetId,
    pub timeline: TimelineId,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SongRecord {
    pub version: u32,
    pub id: SongId,
    pub reference_revision: u64,
    pub reference: Recording,
    pub assets: Vec<AudioAsset>,
    pub timelines: Vec<TimelineRecord>,
    pub recordings: Vec<Recording>,
    pub controls: Controls,
    pub notes: String,
}
impl SongRecord {
    pub fn new(path: String, digest: String, audio: &Audio) -> Self {
        let asset = AudioAsset {
            id: AudioAssetId::new(),
            path,
            digest,
            rate: audio.rate,
            frames: audio.frames.len() as u64,
        };
        let timeline = TimelineRecord::new(audio);
        let reference = Recording {
            asset: asset.id,
            timeline: timeline.id,
        };
        Self {
            version: 1,
            id: SongId::new(),
            reference_revision: 1,
            reference: reference.clone(),
            assets: vec![asset],
            timelines: vec![timeline],
            recordings: vec![reference],
            controls: Controls::default(),
            notes: String::new(),
        }
    }
    pub fn asset(&self) -> Result<&AudioAsset> {
        self.assets
            .iter()
            .find(|a| a.id == self.reference.asset)
            .context("Missing reference asset")
    }
    pub fn timeline(&self) -> Result<&TimelineRecord> {
        self.timelines
            .iter()
            .find(|t| t.id == self.reference.timeline)
            .context("Missing reference timeline")
    }
    pub fn reference(&self, session: SessionId) -> Reference {
        Reference {
            song: self.id,
            asset: self.reference.asset,
            timeline: self.reference.timeline,
            revision: self.reference_revision,
            session,
        }
    }
    pub fn document(&self, session: SessionId) -> Result<Document> {
        let t = self.timeline()?;
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
                && r.asset == self.reference.asset
                && r.timeline == self.reference.timeline
                && r.revision == self.reference_revision,
            "Recording changed; discard stale practice update"
        );
        let t = self.timeline()?;
        d.validate(&self.id.to_string(), t.frames as f64 / t.rate as f64)?;
        self.controls = d.controls;
        self.notes = d.notes.clone();
        let t = self
            .timelines
            .iter_mut()
            .find(|t| t.id == r.timeline)
            .unwrap();
        t.sections = d.sections.clone();
        t.selected_section = d.selected_section.clone();
        t.loop_selection = d.loop_selection;
        t.loop_enabled = d.loop_enabled;
        t.last_position = d.last_position;
        t.measure_map = d.measure_map.clone();
        Ok(())
    }
    pub fn replace(
        &mut self,
        path: String,
        digest: String,
        audio: &Audio,
        timing_unchanged: bool,
    ) -> Result<()> {
        let current = self.timeline()?;
        ensure!(
            !timing_unchanged
                || (current.rate == audio.rate && current.frames == audio.frames.len() as u64),
            "Unchanged timing requires the same decoded sample rate and frame count; choose a new timeline"
        );
        let asset = AudioAsset {
            id: AudioAssetId::new(),
            path,
            digest,
            rate: audio.rate,
            frames: audio.frames.len() as u64,
        };
        let timeline = if timing_unchanged {
            current.id
        } else {
            let t = TimelineRecord::new(audio);
            let id = t.id;
            self.timelines.push(t);
            id
        };
        self.reference = Recording {
            asset: asset.id,
            timeline,
        };
        self.recordings.push(self.reference.clone());
        self.assets.push(asset);
        self.reference_revision = self
            .reference_revision
            .checked_add(1)
            .context("Reference revision exhausted")?;
        self.validate()
    }
    pub fn select_recording(&mut self, asset: AudioAssetId) -> Result<()> {
        self.reference = self
            .recordings
            .iter()
            .find(|r| r.asset == asset)
            .context("Recording is not retained")?
            .clone();
        self.reference_revision = self
            .reference_revision
            .checked_add(1)
            .context("Reference revision exhausted")?;
        self.validate()
    }
    pub fn validate(&self) -> Result<()> {
        ensure!(
            self.version == 1 && self.reference_revision > 0,
            "Unsupported song schema; original file preserved"
        );
        ensure!(
            !self.assets.is_empty()
                && self.assets.len() <= 1000
                && !self.timelines.is_empty()
                && self.timelines.len() <= 1000
                && self.recordings.len() == self.assets.len(),
            "Invalid recording count"
        );
        let mut assets = HashSet::new();
        let mut timelines = HashSet::new();
        for a in &self.assets {
            ensure!(
                assets.insert(a.id)
                    && !a.path.is_empty()
                    && a.path.len() <= 16384
                    && !a.path.contains('\0')
                    && a.digest.len() == 64
                    && a.digest.bytes().all(|b| b.is_ascii_hexdigit())
                    && a.rate > 0
                    && a.frames > 0,
                "Invalid audio asset"
            );
        }
        for t in &self.timelines {
            ensure!(
                t.version == 1 && timelines.insert(t.id) && t.rate > 0 && t.frames > 0,
                "Invalid timeline"
            );
            let mut d = Document::new(self.id.to_string());
            d.controls = self.controls;
            d.notes = self.notes.clone();
            d.sections = t.sections.clone();
            d.selected_section = t.selected_section.clone();
            d.loop_selection = t.loop_selection;
            d.loop_enabled = t.loop_enabled;
            d.last_position = t.last_position;
            d.measure_map = t.measure_map.clone();
            d.validate(&self.id.to_string(), t.frames as f64 / t.rate as f64)?;
            ensure!(
                t.measure_map
                    .as_ref()
                    .is_none_or(|m| m.rate == t.rate && m.frames == t.frames),
                "Map geometry does not match timeline"
            );
        }
        let mut refs = HashSet::new();
        for r in &self.recordings {
            ensure!(
                refs.insert(r.asset)
                    && assets.contains(&r.asset)
                    && timelines.contains(&r.timeline),
                "Invalid recording reference"
            );
            let a = self.assets.iter().find(|a| a.id == r.asset).unwrap();
            let t = self.timelines.iter().find(|t| t.id == r.timeline).unwrap();
            ensure!(
                a.rate == t.rate && a.frames == t.frames,
                "Recording geometry differs from timeline"
            );
        }
        ensure!(
            self.recordings
                .iter()
                .any(|r| r.asset == self.reference.asset && r.timeline == self.reference.timeline),
            "Missing selected recording"
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
}

#[derive(Clone, Serialize)]
pub struct RecordingOption {
    pub asset_id: AudioAssetId,
    pub timeline_id: TimelineId,
    pub path: String,
    pub current: bool,
}
impl SongRecord {
    pub fn recording_options(&self) -> Vec<RecordingOption> {
        self.recordings
            .iter()
            .map(|r| RecordingOption {
                asset_id: r.asset,
                timeline_id: r.timeline,
                path: self
                    .assets
                    .iter()
                    .find(|a| a.id == r.asset)
                    .unwrap()
                    .path
                    .clone(),
                current: r.asset == self.reference.asset,
            })
            .collect()
    }
}
