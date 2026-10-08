use anyhow::{Result, ensure};
use std::{ffi::c_void, ptr::NonNull};

pub const BLOCK: usize = 512;

// The declarations match the pinned upstream rubberband/rubberband-c.h.
unsafe extern "C" {
    fn rubberband_new(
        rate: u32,
        channels: u32,
        options: i32,
        ratio: f64,
        pitch: f64,
    ) -> *mut c_void;
    fn rubberband_delete(state: *mut c_void);
    fn rubberband_set_time_ratio(state: *mut c_void, ratio: f64);
    fn rubberband_set_pitch_scale(state: *mut c_void, scale: f64);
    fn rubberband_set_max_process_size(state: *mut c_void, frames: u32);
    fn rubberband_get_preferred_start_pad(state: *mut c_void) -> u32;
    fn rubberband_get_start_delay(state: *mut c_void) -> u32;
    fn rubberband_get_engine_version(state: *mut c_void) -> i32;
    fn rubberband_process(
        state: *mut c_void,
        input: *const *const f32,
        frames: u32,
        final_block: i32,
    );
    fn rubberband_available(state: *mut c_void) -> i32;
    fn rubberband_retrieve(state: *mut c_void, output: *const *mut f32, frames: u32) -> u32;
}

#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Controls {
    /// Source seconds per output second. 0.5 means half-speed.
    pub speed: f64,
    pub semitones: f64,
}

impl Default for Controls {
    fn default() -> Self {
        Self {
            speed: 1.0,
            semitones: 0.0,
        }
    }
}

impl Controls {
    pub fn validate(self) -> Result<Self> {
        ensure!(
            self.speed.is_finite() && (0.25..=2.0).contains(&self.speed),
            "Speed must be 0.25–2.0"
        );
        ensure!(
            self.semitones.is_finite() && (-12.0..=12.0).contains(&self.semitones),
            "Pitch must be -12–12 semitones"
        );
        Ok(self)
    }
    pub fn pitch_scale(self) -> f64 {
        2_f64.powf(self.semitones / 12.0)
    }
}

/// Single-owner, worker-thread-only wrapper. No unsafe Send or Sync promises.
pub struct Dsp {
    state: NonNull<c_void>,
    pub pad: usize,
    pub delay: usize,
    pub engine_version: i32,
}

impl Dsp {
    pub fn new(rate: u32, controls: Controls, finer: bool) -> Result<Self> {
        controls.validate()?;
        ensure!(
            (8_000..=192_000).contains(&rate),
            "DSP rate must be 8000–192000 Hz"
        );
        // RealTime | ThreadingNever | PitchHighConsistency | ChannelsTogether.
        // R3 uses WindowShort for latency; R2 keeps its standard window.
        // Using R2's short window produced unstable zero-crossing pitch checks.
        let options =
            0x1 | 0x10000 | 0x4000000 | 0x10000000 | if finer { 0x20000000 | 0x100000 } else { 0 };
        // SAFETY: valid rate, two channels, validated positive ratios, and known
        // option bits. Ownership of the returned state stays in this wrapper.
        let state = NonNull::new(unsafe {
            rubberband_new(
                rate,
                2,
                options,
                1.0 / controls.speed,
                controls.pitch_scale(),
            )
        })
        .ok_or_else(|| anyhow::anyhow!("Rubber Band allocation failed"))?;
        // SAFETY: state is live and exclusively owned here.
        unsafe { rubberband_set_max_process_size(state.as_ptr(), BLOCK as u32) };
        let (pad, delay, engine_version) = unsafe {
            (
                rubberband_get_preferred_start_pad(state.as_ptr()) as usize,
                rubberband_get_start_delay(state.as_ptr()) as usize,
                rubberband_get_engine_version(state.as_ptr()),
            )
        };
        Ok(Self {
            state,
            pad,
            delay,
            engine_version,
        })
    }

    pub fn set_controls(&mut self, controls: Controls) -> Result<()> {
        controls.validate()?;
        // SAFETY: mutable access ensures no concurrent native call.
        unsafe {
            rubberband_set_time_ratio(self.state.as_ptr(), 1.0 / controls.speed);
            rubberband_set_pitch_scale(self.state.as_ptr(), controls.pitch_scale());
        }
        Ok(())
    }

    pub fn process(&mut self, left: &[f32], right: &[f32], final_block: bool) {
        assert_eq!(left.len(), right.len());
        assert!(left.len() <= BLOCK);
        let pointers = [left.as_ptr(), right.as_ptr()];
        // SAFETY: both planar slices contain the advertised number of frames,
        // and Rubber Band consumes them synchronously without retaining them.
        unsafe {
            rubberband_process(
                self.state.as_ptr(),
                pointers.as_ptr(),
                left.len() as u32,
                final_block as i32,
            )
        };
    }

    pub fn available(&self) -> i32 {
        // SAFETY: state remains live; callers run on the owning worker.
        unsafe { rubberband_available(self.state.as_ptr()) }
    }

    pub fn retrieve(&mut self, left: &mut [f32], right: &mut [f32]) -> usize {
        assert_eq!(left.len(), right.len());
        let count = left.len().min(self.available().max(0) as usize);
        if count == 0 {
            return 0;
        }
        let pointers = [left.as_mut_ptr(), right.as_mut_ptr()];
        // SAFETY: separate writable planes, bounded by both output capacity
        // and available native frames; state is exclusively borrowed.
        unsafe {
            rubberband_retrieve(self.state.as_ptr(), pointers.as_ptr(), count as u32) as usize
        }
    }
}

impl Drop for Dsp {
    fn drop(&mut self) {
        // SAFETY: this wrapper owns exactly one native allocation.
        unsafe { rubberband_delete(self.state.as_ptr()) };
    }
}
