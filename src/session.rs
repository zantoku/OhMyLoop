use crate::{dsp::Controls, engine::LoopRegion};
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, path::Path};

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Session {
    pub version: u32,
    pub song: String,
    pub controls: Controls,
    pub markers: BTreeMap<String, f64>,
    pub active_loop: Option<LoopRegion>,
}

impl Session {
    pub fn new(song: String) -> Self {
        Self {
            version: 1,
            song,
            controls: Controls::default(),
            markers: BTreeMap::new(),
            active_loop: None,
        }
    }

    pub fn validate(&self, duration: f64) -> Result<()> {
        ensure!(self.version == 1, "Unsupported session version");
        self.controls.validate()?;
        for (name, position) in &self.markers {
            ensure!(
                !name.trim().is_empty() && name.len() <= 128,
                "Invalid marker name"
            );
            ensure!(
                position.is_finite() && (0.0..=duration).contains(position),
                "Marker outside song"
            );
        }
        if let Some(region) = self.active_loop {
            region.validate(duration)?;
        }
        Ok(())
    }

    pub fn save(&self, path: &Path, duration: f64) -> Result<()> {
        self.validate(duration)?;
        let json = serde_json::to_string_pretty(self)?;
        std::fs::write(path, json)?;
        Ok(())
    }

    pub fn load(path: &Path, song: &str, duration: f64) -> Result<Self> {
        let session: Self = serde_json::from_slice(&std::fs::read(path)?)?;
        session.validate(duration)?;
        ensure!(session.song == song, "Session belongs to a different song");
        Ok(session)
    }
}
