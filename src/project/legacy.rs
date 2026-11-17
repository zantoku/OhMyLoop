//! Read old UUID envelopes by keeping only their selected recording and timeline.
use super::*;
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct LegacyRecording {
    asset: AudioAssetId,
    timeline: TimelineId,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct LegacySongRecord {
    version: u32,
    id: SongId,
    reference_revision: u64,
    reference: LegacyRecording,
    assets: Vec<AudioAsset>,
    timelines: Vec<TimelineRecord>,
    recordings: Vec<LegacyRecording>,
    controls: Controls,
    notes: String,
}
impl LegacySongRecord {
    pub(super) fn into_current(self) -> Result<SongRecord> {
        self.validate()?;
        let asset = self
            .assets
            .into_iter()
            .find(|a| a.id == self.reference.asset)
            .context("Missing reference asset")?;
        let timeline = self
            .timelines
            .into_iter()
            .find(|t| t.id == self.reference.timeline)
            .context("Missing reference timeline")?;
        Ok(SongRecord {
            version: 2,
            id: self.id,
            reference_revision: self.reference_revision,
            asset,
            timeline,
            controls: self.controls,
            notes: self.notes,
        })
    }
    fn validate(&self) -> Result<()> {
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
        for asset in &self.assets {
            ensure!(assets.insert(asset.id), "Duplicate legacy audio asset");
            asset.validate()?;
        }
        for timeline in &self.timelines {
            ensure!(timelines.insert(timeline.id), "Duplicate legacy timeline");
            timeline.validate(self.id, self.controls, &self.notes)?;
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
