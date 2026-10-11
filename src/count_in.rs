//! One-measure count-ins rendered on the audio worker, in output frames.
use crate::{
    metronome::{ClickMixer, Tick},
    timing::{Accent, MeasureMap},
};
use anyhow::{Context, Result, ensure};
use std::collections::HashMap;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Settings {
    pub enabled: bool,
    pub repeat: bool,
    pub volume: f32,
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            enabled: false,
            repeat: false,
            volume: 0.5,
        }
    }
}
impl Settings {
    pub fn validate(self) -> Result<Self> {
        ensure!(
            self.volume.is_finite() && (0.0..=1.0).contains(&self.volume),
            "Invalid count-in volume"
        );
        Ok(self)
    }
}

#[derive(Clone)]
pub struct Plan {
    pub length: f64,
    pub ticks: Vec<Tick>,
}
impl Plan {
    /// Extrapolate the local anchor interval for one full notated measure.
    /// Outside the map, use the nearest mapped frame. Preserve musical phase
    /// at pickups and arbitrary starts, so the last interval leads into playback.
    pub fn at(map: &MeasureMap, seconds: f64, rate: u32) -> Result<Self> {
        map.validate()?;
        ensure!(
            seconds.is_finite() && seconds >= 0.0 && rate > 0,
            "Invalid count-in position or sample rate"
        );
        let requested = (seconds * f64::from(map.rate)).round() as u64;
        let bounds: HashMap<_, _> = map
            .boundaries
            .iter()
            .map(|b| (b.id.as_str(), b.frame))
            .collect();
        let (measure, frame) = map
            .measures
            .iter()
            .map(|m| {
                let frame = requested.clamp(bounds[m.start.as_str()], bounds[m.end.as_str()] - 1);
                (m, frame)
            })
            .min_by_key(|(_, frame)| requested.abs_diff(*frame))
            .context("Build a measure map before enabling count-in")?;
        let position = map
            .position(frame)
            .context("Count-in position is not mapped")?;
        let meter = &measure.meter;
        let pulse = meter.pulse.value();
        let length = meter.length().value();
        let frames_per_quarter = 60.0 * f64::from(rate) / (position.tempo_bpm * pulse);
        let phase = (position.beat - 1.0) * pulse;
        let count = (length / pulse).ceil() as usize;
        ensure!(
            count <= 512,
            "Choose a larger pulse note value for the count-in"
        );
        let mut groups = vec![];
        let mut at = 0.0;
        for group in &meter.grouping {
            groups.push(at);
            at += f64::from(*group) * 4.0 / f64::from(meter.denominator);
        }
        let mut ticks = vec![];
        for index in 0..count {
            let quarter = index as f64 * pulse;
            let offset = (quarter - phase).rem_euclid(length);
            let accent = if index == 0 {
                Accent::Measure
            } else if groups.iter().any(|g| (g - quarter).abs() < 1e-9) {
                Accent::Group
            } else {
                Accent::Beat
            };
            ticks.push(Tick {
                source_frame: offset * frames_per_quarter,
                accent,
            });
        }
        ticks.sort_by(|a, b| a.source_frame.total_cmp(&b.source_frame));
        Ok(Self {
            length: length * frames_per_quarter,
            ticks,
        })
    }
}

pub struct CountIn {
    length: f64,
    rate: u32,
    mixer: ClickMixer,
    position: f64,
    pub source_frame: f64,
}
impl CountIn {
    pub fn new(plan: Plan, rate: u32, volume: f32, source_frame: f64) -> Self {
        let mut mixer = ClickMixer::new(rate);
        mixer.configure(true, volume, plan.ticks);
        Self {
            length: plan.length,
            rate,
            mixer,
            position: 0.0,
            source_frame,
        }
    }
    pub fn set_volume(&mut self, volume: f32) {
        self.mixer.set_volume(volume);
    }
    pub fn next(&mut self, speed: f64) -> Option<[f32; 2]> {
        if self.position >= self.length - 1e-9 {
            return None;
        }
        let mut samples = self.mixer.mix(self.position, [0.0; 2]);
        // A start inside a pulse can leave a click tail at the join. Fade it
        // over the last five output milliseconds instead of cutting it off.
        let gain = ((self.length - self.position) / (speed * f64::from(self.rate) * 0.005)).min(1.0)
            as f32;
        samples.iter_mut().for_each(|sample| *sample *= gain);
        self.position += speed;
        Some(samples)
    }
}
