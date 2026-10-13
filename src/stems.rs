//! Bounded, temporary PCM sources for reference/stem comparison.
//!
//! All preparation and reads belong on a worker, never the device callback.
mod prepare;

use anyhow::{Result, ensure};
pub use prepare::{PreparationProgress, PreparedComparison, library};
use std::{
    fs::File,
    io::{Read, Seek, SeekFrom},
    path::PathBuf,
    sync::Arc,
};

pub trait BlockSource: Send {
    fn rate(&self) -> u32;
    fn len_frames(&self) -> u64;
    fn read_frames(&mut self, position: u64, output: &mut [[f32; 2]]) -> Result<usize>;
    /// Update aligned input mixing without discarding DSP history when supported.
    fn update_mix(&mut self, _selection: usize, _mix: MixState) -> Result<bool> {
        Ok(false)
    }
}

pub trait SourceFactory: Send + Sync {
    fn rate(&self) -> u32;
    fn len_frames(&self) -> u64;
    fn open(&self, selection: usize, mix: MixState) -> Result<Box<dyn BlockSource>>;
    /// Canonical recording coordinates, before device-rate rounding.
    fn reference_geometry(&self) -> (u32, u64) {
        (self.rate(), self.len_frames())
    }
    fn resample_legacy(&self, _rate: u32) -> Result<Arc<dyn SourceFactory>> {
        anyhow::bail!("Prepare bounded PCM at output device rate")
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Role {
    Bass,
    Drums,
    Guitar,
    Piano,
    Vocals,
    Other,
}
impl Role {
    pub const ALL: [Self; 6] = [
        Self::Bass,
        Self::Drums,
        Self::Guitar,
        Self::Piano,
        Self::Vocals,
        Self::Other,
    ];
    pub fn name(self) -> &'static str {
        match self {
            Self::Bass => "bass",
            Self::Drums => "drums",
            Self::Guitar => "guitar",
            Self::Piano => "piano",
            Self::Vocals => "vocals",
            Self::Other => "other",
        }
    }
    pub fn index(self) -> usize {
        self as usize
    }
    pub fn parse(name: &str) -> Result<Self> {
        Self::ALL
            .into_iter()
            .find(|r| r.name() == name)
            .ok_or_else(|| anyhow::anyhow!("Unknown stem role: {name}"))
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RoleControl {
    pub gain: f32,
    pub mute: bool,
    pub solo: bool,
}
impl Default for RoleControl {
    fn default() -> Self {
        Self {
            gain: 1.0,
            mute: false,
            solo: false,
        }
    }
}
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct MixState {
    pub tracks: [RoleControl; 6],
}
impl MixState {
    pub fn validate(self) -> Result<()> {
        ensure!(
            self.tracks
                .iter()
                .all(|x| x.gain.is_finite() && (0.0..=1.0).contains(&x.gain)),
            "Track gain must be finite and between 0 and 1"
        );
        Ok(())
    }
    pub fn gain(self, role: Role) -> f32 {
        let track = self.tracks[role.index()];
        if track.mute || (self.tracks.iter().any(|x| x.solo) && !track.solo) {
            0.0
        } else {
            track.gain
        }
    }
}

pub(crate) struct TemporaryDirectory(pub PathBuf, pub std::sync::atomic::AtomicBool);
impl Drop for TemporaryDirectory {
    fn drop(&mut self) {
        if !self.1.load(std::sync::atomic::Ordering::Acquire) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }
}
#[derive(Clone)]
pub(crate) struct Pcm {
    pub path: PathBuf,
    pub rate: u32,
    pub frames: u64,
    pub channels: u32,
}
pub(crate) struct PcmReader {
    file: File,
    pcm: Pcm,
    bytes: Vec<u8>,
}
impl PcmReader {
    pub(crate) fn new(pcm: Pcm) -> Result<Self> {
        Ok(Self {
            file: File::open(&pcm.path)?,
            pcm,
            bytes: vec![0; 8192 * 8],
        })
    }
    pub(crate) fn read(&mut self, position: u64, out: &mut [[f32; 2]]) -> Result<usize> {
        let count = (self.pcm.frames.saturating_sub(position)).min(out.len() as u64) as usize;
        if count == 0 {
            return Ok(0);
        }
        self.file.seek(SeekFrom::Start(
            position
                .checked_mul(8)
                .ok_or_else(|| anyhow::anyhow!("PCM position overflow"))?,
        ))?;
        for chunk in out[..count].chunks_mut(8192) {
            let bytes = &mut self.bytes[..chunk.len() * 8];
            self.file.read_exact(bytes)?;
            for (frame, b) in chunk.iter_mut().zip(bytes.as_chunks::<8>().0) {
                *frame = [
                    f32::from_le_bytes(b[..4].try_into()?),
                    f32::from_le_bytes(b[4..].try_into()?),
                ];
                ensure!(
                    frame.iter().all(|x| x.is_finite()),
                    "Invalid prepared PCM sample"
                );
            }
        }
        Ok(count)
    }
}

// One reference plus the two six-role banks supported by the listening harness.
const MIX_COMPONENTS: usize = 13;
pub(crate) struct GainRamp {
    current: [f32; MIX_COMPONENTS],
    target: [f32; MIX_COMPONENTS],
    remaining: usize,
}
impl Default for GainRamp {
    fn default() -> Self {
        let mut weights = [0.; MIX_COMPONENTS];
        weights[0] = 1.;
        Self {
            current: weights,
            target: weights,
            remaining: 0,
        }
    }
}
impl GainRamp {
    fn gain(&self, component: usize, offset: usize) -> f32 {
        if self.remaining == 0 {
            return self.target[component];
        }
        let fraction = (offset + 1).min(self.remaining) as f32 / self.remaining as f32;
        self.current[component] + (self.target[component] - self.current[component]) * fraction
    }
    fn advance(&mut self, frames: usize) {
        if self.remaining == 0 {
            return;
        }
        let fraction = frames.min(self.remaining) as f32 / self.remaining as f32;
        for (value, target) in self.current.iter_mut().zip(self.target) {
            *value += (target - *value) * fraction;
        }
        self.remaining = self.remaining.saturating_sub(frames);
    }
}

pub struct StemSource {
    reference: PcmReader,
    sets: Vec<Vec<(Role, PcmReader)>>,
    selection: usize,
    mix: MixState,
    attenuation: f32,
    scratch: Vec<[f32; 2]>,
    smoothing: GainRamp,
    _temporary: Arc<TemporaryDirectory>,
}
impl StemSource {
    pub fn select_source(&mut self, index: usize) -> Result<()> {
        ensure!(index <= self.sets.len(), "Source index out of range");
        self.selection = index;
        self.instant_mix();
        Ok(())
    }
    pub fn set_mix(&mut self, mix: MixState) -> Result<()> {
        mix.validate()?;
        self.mix = mix;
        self.instant_mix();
        Ok(())
    }
    fn weights(&self) -> [f32; MIX_COMPONENTS] {
        let mut weights = [0.; MIX_COMPONENTS];
        if self.selection == 0 {
            weights[0] = 1.;
        } else {
            for (role, _) in &self.sets[self.selection - 1] {
                weights[1 + (self.selection - 1) * 6 + role.index()] = self.mix.gain(*role);
            }
        }
        weights
    }
    fn instant_mix(&mut self) {
        let weights = self.weights();
        self.smoothing = GainRamp {
            current: weights,
            target: weights,
            remaining: 0,
        };
    }
    pub fn source_count(&self) -> usize {
        self.sets.len() + 1
    }
    pub fn source_roles(&self, index: usize) -> Result<Vec<Role>> {
        ensure!(index <= self.sets.len(), "Source index out of range");
        Ok(if index == 0 {
            vec![]
        } else {
            self.sets[index - 1].iter().map(|(r, _)| *r).collect()
        })
    }
}
impl BlockSource for StemSource {
    fn rate(&self) -> u32 {
        self.reference.pcm.rate
    }
    fn len_frames(&self) -> u64 {
        self.reference.pcm.frames
    }
    fn update_mix(&mut self, selection: usize, mix: MixState) -> Result<bool> {
        ensure!(selection <= self.sets.len(), "Source index out of range");
        mix.validate()?;
        self.selection = selection;
        self.mix = mix;
        self.smoothing.target = self.weights();
        self.smoothing.remaining = (self.rate() as usize / 200).max(1);
        Ok(true)
    }
    fn read_frames(&mut self, position: u64, output: &mut [[f32; 2]]) -> Result<usize> {
        let count = self
            .len_frames()
            .saturating_sub(position)
            .min(output.len() as u64) as usize;
        if self.selection == 0 && self.smoothing.remaining == 0 {
            self.reference.read(position, &mut output[..count])?;
            for frame in &mut output[..count] {
                for x in frame {
                    *x *= self.attenuation
                }
            }
            return Ok(count);
        }
        for (block_index, block) in output[..count].chunks_mut(8192).enumerate() {
            block.fill([0.0; 2]);
            let pos = position + block_index as u64 * 8192;
            if self.smoothing.current[0] != 0. || self.smoothing.target[0] != 0. {
                ensure!(
                    self.reference.read(pos, &mut self.scratch[..block.len()])? == block.len(),
                    "Unexpected reference EOF"
                );
                for (offset, (out, input)) in block.iter_mut().zip(&self.scratch).enumerate() {
                    let gain = self.smoothing.gain(0, offset) * self.attenuation;
                    out[0] += input[0] * gain;
                    out[1] += input[1] * gain;
                }
            }
            for (bank_index, bank) in self.sets.iter_mut().enumerate() {
                let base = 1 + bank_index * 6;
                let audible = bank.iter().any(|(role, _)| {
                    self.smoothing.current[base + role.index()] != 0.
                        || self.smoothing.target[base + role.index()] != 0.
                });
                if self.selection != bank_index + 1 && !audible {
                    continue;
                }
                for (role, reader) in bank {
                    // Check every selected-bank file, including muted tracks.
                    ensure!(
                        reader.read(pos, &mut self.scratch[..block.len()])? == block.len(),
                        "Unexpected stem EOF"
                    );
                    for (offset, (out, input)) in block.iter_mut().zip(&self.scratch).enumerate() {
                        let gain =
                            self.smoothing.gain(base + role.index(), offset) * self.attenuation;
                        out[0] += input[0] * gain;
                        out[1] += input[1] * gain;
                    }
                }
            }
            self.smoothing.advance(block.len());
        }
        Ok(count)
    }
}
