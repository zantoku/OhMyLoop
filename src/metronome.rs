//! Post-DSP click mixing on the render worker. No UI clocks or callback work.
use crate::{
    engine::LoopRegion,
    timing::{Accent, MeasureMap},
};
use anyhow::{Result, ensure};
#[derive(Clone, Copy, Debug)]
pub struct Tick {
    pub source_frame: f64,
    pub accent: Accent,
}
pub fn ticks(map: &MeasureMap, device_rate: u32) -> Result<Vec<Tick>> {
    ensure!(device_rate > 0, "Invalid output sample rate");
    let events = map.pulse_events()?;
    let end = (u128::from(map.frames) * u128::from(device_rate) / u128::from(map.rate)) as f64;
    let mut ticks: Vec<_> = events
        .into_iter()
        .map(|p| Tick {
            source_frame: (p.frame as f64 * f64::from(device_rate) / f64::from(map.rate)).round(),
            accent: p.accent,
        })
        .filter(|p| p.source_frame < end)
        .collect();
    // Match Audio::resample's truncated output length and exclusive EOF.
    // Seeks and loops also round at the device-rate PCM boundary. Keep a
    // downbeat audible when its exact original frame converts to a fraction.
    ticks.dedup_by(|later, earlier| {
        if later.source_frame == earlier.source_frame {
            earlier.accent = earlier.accent.max(later.accent);
            true
        } else {
            false
        }
    });
    Ok(ticks)
}
/// Each click has a fixed output duration/pitch, independent of musical speed.
pub struct ClickMixer {
    rate: u32,
    ticks: Vec<Tick>,
    cursor: usize,
    previous: Option<f64>,
    loop_start: Option<f64>,
    waveforms: [Vec<f32>; 3],
    voice: Option<(usize, usize)>,
    enabled: bool,
    volume: f32,
    gain: f32,
    click_gain: f32,
    slew: f32,
}
impl ClickMixer {
    pub fn new(rate: u32) -> Self {
        let waveforms = [1100.0, 1450.0, 1850.0].map(|hz| {
            (0..(rate as usize * 24 / 1000).max(1))
                .map(|n| {
                    let t = n as f32 / rate as f32;
                    let attack = (t / 0.0005).min(1.0);
                    (std::f32::consts::TAU * hz * t).sin() * (-t * 200.0).exp() * attack
                })
                .collect()
        });
        Self {
            rate,
            ticks: vec![],
            cursor: 0,
            previous: None,
            loop_start: None,
            waveforms,
            voice: None,
            enabled: false,
            volume: 0.5,
            gain: 1.0,
            click_gain: 0.0,
            slew: 1.0 / (rate as f32 * 0.005).max(1.0),
        }
    }
    pub fn configure(&mut self, enabled: bool, volume: f32, ticks: Vec<Tick>) {
        self.enabled = enabled;
        self.volume = volume;
        self.ticks = ticks;
        self.previous = None;
        // A short existing click may decay naturally across a schedule change.
    }
    pub fn reset(&mut self, region: Option<LoopRegion>) {
        self.previous = None;
        self.voice = None;
        // Engine rounds source loop bounds to whole device-rate PCM frames.
        self.loop_start = region.map(|r| (r.start * f64::from(self.rate)).round());
    }
    pub fn mix(&mut self, frame: f64, audio: [f32; 2]) -> [f32; 2] {
        if self.previous.is_none() || self.previous.is_some_and(|last| frame < last) {
            let start = if self.previous.is_some() {
                self.loop_start.unwrap_or(frame)
            } else {
                frame
            };
            self.cursor = self
                .ticks
                .partition_point(|t| t.source_frame < start - 1e-9);
            if self.previous.is_some() {
                self.voice = None;
            }
        }
        self.previous = Some(frame);
        if self.enabled {
            while let Some(tick) = self
                .ticks
                .get(self.cursor)
                .filter(|t| t.source_frame <= frame + 1e-9)
            {
                self.voice = Some((tick.accent as usize, 0));
                self.cursor += 1;
            }
        }
        let target = if self.enabled { 0.4 * self.volume } else { 0.0 };
        self.click_gain += (target - self.click_gain).clamp(-self.slew, self.slew);
        let target_gain = 1.0 - target;
        self.gain += (target_gain - self.gain).clamp(-self.slew, self.slew);
        let mut click = 0.0;
        if let Some((accent, at)) = self.voice {
            if let Some(sample) = self.waveforms[accent].get(at) {
                click = sample * self.click_gain;
                self.voice = Some((accent, at + 1));
            } else {
                self.voice = None;
            }
        }
        [audio[0] * self.gain + click, audio[1] * self.gain + click]
    }
}
