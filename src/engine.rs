use crate::{
    audio::Audio,
    dsp::{BLOCK, Controls, Dsp},
};
use anyhow::{Result, ensure};
use std::sync::Arc;

#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct LoopRegion {
    pub start: f64,
    pub end: f64,
    pub fade_ms: f64,
}

impl LoopRegion {
    pub fn validate(self, duration: f64) -> Result<Self> {
        ensure!(
            self.start.is_finite() && self.end.is_finite(),
            "Loop bounds must be finite"
        );
        ensure!(
            self.start >= 0.0 && self.end <= duration && self.end - self.start + 1e-9 >= 0.1,
            "Loop must be inside the song and at least 100 ms long"
        );
        ensure!(
            self.fade_ms.is_finite() && (0.0..=50.0).contains(&self.fade_ms),
            "Fade must be 0–50 ms"
        );
        Ok(self)
    }
}

#[derive(Clone, Copy, Debug)]
pub struct RenderedFrame {
    pub samples: [f32; 2],
    /// Nominal source coordinate. Rubber Band's adaptive timing means that
    /// this is an estimate, not a sample-accurate analysis of its output.
    pub source_frame: f64,
}

pub struct Engine {
    audio: Arc<Audio>,
    dsp: Dsp,
    controls: Controls,
    region: Option<LoopRegion>,
    input_cursor: usize,
    source_position: f64,
    pad_remaining: usize,
    discard_remaining: usize,
    finalized: bool,
    pub loop_wraps: u64,
    left: [f32; BLOCK],
    right: [f32; BLOCK],
}

impl Engine {
    pub fn new(
        audio: Arc<Audio>,
        controls: Controls,
        position: f64,
        region: Option<LoopRegion>,
        finer: bool,
    ) -> Result<Self> {
        controls.validate()?;
        ensure!(
            position.is_finite() && (0.0..=audio.duration()).contains(&position),
            "Seek must be inside song"
        );
        if let Some(region) = region {
            region.validate(audio.duration())?;
        }
        let position = match region {
            Some(r) if position < r.start || position >= r.end => r.start,
            _ => position,
        };
        let input_cursor = (position * audio.rate as f64).round() as usize;
        let dsp = Dsp::new(audio.rate, controls, finer)?;
        let pad_remaining = dsp.pad;
        let discard_remaining = dsp.delay;
        Ok(Self {
            audio,
            dsp,
            controls,
            region,
            input_cursor,
            source_position: input_cursor as f64,
            pad_remaining,
            discard_remaining,
            finalized: false,
            loop_wraps: 0,
            left: [0.0; BLOCK],
            right: [0.0; BLOCK],
        })
    }

    pub fn set_controls(&mut self, controls: Controls) -> Result<()> {
        self.dsp.set_controls(controls)?;
        self.controls = controls;
        Ok(())
    }

    pub fn startup(&self) -> (usize, usize, i32) {
        (self.dsp.pad, self.dsp.delay, self.dsp.engine_version)
    }

    pub fn input_position(&self) -> f64 {
        self.input_cursor as f64 / self.audio.rate as f64
    }

    pub fn output_position(&self) -> f64 {
        self.source_position / self.audio.rate as f64
    }

    fn loop_frames(&self) -> Option<(usize, usize, usize)> {
        self.region.map(|r| {
            let start = (r.start * self.audio.rate as f64).round() as usize;
            let end = (r.end * self.audio.rate as f64).round() as usize;
            let fade = ((r.fade_ms * self.audio.rate as f64 / 1000.0).round() as usize)
                .min((end - start) / 4);
            (start, end, fade)
        })
    }

    fn feed(&mut self) {
        if self.pad_remaining > 0 {
            let count = self.pad_remaining.min(BLOCK);
            self.left[..count].fill(0.0);
            self.right[..count].fill(0.0);
            self.dsp
                .process(&self.left[..count], &self.right[..count], false);
            self.pad_remaining -= count;
            return;
        }
        let mut count = 0;
        let region = self.loop_frames();
        while count < BLOCK {
            if let Some((start, end, _)) = region {
                if self.input_cursor >= end {
                    self.input_cursor = start;
                    self.loop_wraps += 1;
                }
            } else if self.input_cursor >= self.audio.frames.len() {
                break;
            }
            let mut gain = 1.0;
            if let Some((start, end, fade)) = region
                && fade > 0
            {
                // Paired fades preserve the selected loop's exact period.
                // They reduce discontinuities without skipping head samples.
                gain = ((self.input_cursor - start).min(end - 1 - self.input_cursor) as f32
                    / fade as f32)
                    .min(1.0);
            }
            let frame = self.audio.frames[self.input_cursor];
            self.left[count] = frame[0] * gain;
            self.right[count] = frame[1] * gain;
            self.input_cursor += 1;
            count += 1;
        }
        let final_block = region.is_none() && self.input_cursor == self.audio.frames.len();
        self.dsp
            .process(&self.left[..count], &self.right[..count], final_block);
        self.finalized = final_block;
    }

    /// Variable-output streaming. Call on a worker, never in a device callback.
    pub fn next_block(&mut self) -> Vec<RenderedFrame> {
        loop {
            if self.dsp.available() > 0 {
                let count = self.dsp.retrieve(&mut self.left, &mut self.right);
                let discard = count.min(self.discard_remaining);
                self.discard_remaining -= discard;
                let mut result = Vec::with_capacity(count - discard);
                let region = self.loop_frames();
                for n in discard..count {
                    if region.is_none() && self.source_position >= self.audio.frames.len() as f64 {
                        break;
                    }
                    result.push(RenderedFrame {
                        samples: [self.left[n], self.right[n]],
                        source_frame: self.source_position,
                    });
                    self.source_position += self.controls.speed;
                    if let Some((start, end, _)) = region
                        && self.source_position >= end as f64
                    {
                        self.source_position = start as f64
                            + (self.source_position - end as f64) % (end - start) as f64;
                    }
                }
                if !result.is_empty() {
                    return result;
                }
                if self.finalized && self.dsp.available() <= 0 {
                    return Vec::new();
                }
            } else if self.finalized {
                return Vec::new();
            } else {
                self.feed();
            }
        }
    }
}

pub fn render(
    audio: Arc<Audio>,
    controls: Controls,
    position: f64,
    region: Option<LoopRegion>,
    output_frames: Option<usize>,
    finer: bool,
) -> Result<Vec<[f32; 2]>> {
    ensure!(
        region.is_none() || output_frames.is_some(),
        "Loop rendering needs an output limit"
    );
    let mut engine = Engine::new(audio, controls, position, region, finer)?;
    let mut frames = Vec::new();
    loop {
        let block = engine.next_block();
        if block.is_empty() {
            break;
        }
        let remaining = output_frames.map_or(block.len(), |limit| {
            limit.saturating_sub(frames.len()).min(block.len())
        });
        frames.extend(block[..remaining].iter().map(|f| f.samples));
        if output_frames.is_some_and(|limit| frames.len() >= limit) {
            break;
        }
    }
    Ok(frames)
}
