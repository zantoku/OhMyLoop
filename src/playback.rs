use crate::{
    audio::Audio,
    dsp::Controls,
    engine::{Engine, LoopRegion, RenderedFrame},
};
use anyhow::{Context, Result, ensure};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use crossbeam_queue::ArrayQueue;
use std::{
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicU64, Ordering},
        mpsc::{self, Receiver, SyncSender},
    },
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

#[derive(Clone, Copy)]
pub struct QueuedFrame {
    pub frame: RenderedFrame,
    pub epoch: u64,
}

pub struct Shared {
    pub queue: ArrayQueue<QueuedFrame>,
    pub epoch: AtomicU64,
    pub playing: AtomicBool,
    pub finished: AtomicBool,
    pub source_position: AtomicU64,
    pub render_position: AtomicU64,
    pub underflow_frames: AtomicU64,
    pub callbacks: AtomicU64,
    pub output_frames: AtomicU64,
    pub device_errors: AtomicU64,
    pub max_render_us: AtomicU64,
    pub last_seek_us: AtomicU64,
    pub loop_wraps: AtomicU64,
    pub worker_error: Mutex<Option<String>>,
}

impl Shared {
    pub fn new(capacity: usize) -> Self {
        Self {
            queue: ArrayQueue::new(capacity),
            epoch: AtomicU64::new(0),
            playing: AtomicBool::new(false),
            finished: AtomicBool::new(false),
            source_position: AtomicU64::new(0_f64.to_bits()),
            render_position: AtomicU64::new(0_f64.to_bits()),
            underflow_frames: AtomicU64::new(0),
            callbacks: AtomicU64::new(0),
            output_frames: AtomicU64::new(0),
            device_errors: AtomicU64::new(0),
            max_render_us: AtomicU64::new(0),
            last_seek_us: AtomicU64::new(0),
            loop_wraps: AtomicU64::new(0),
            worker_error: Mutex::new(None),
        }
    }
    pub fn position(&self) -> f64 {
        f64::from_bits(self.source_position.load(Ordering::Relaxed))
    }
}

/// The exact code exercised by the CPAL callback and deterministic queue tests.
/// Only fixed arrays, lock-free queue operations, and scalar atomics are used.
pub struct Consumer {
    shared: Arc<Shared>,
    epoch: u64,
    last: [f32; 2],
    fade_remaining: usize,
    fade_length: usize,
    fade_in: usize,
    was_playing: bool,
}

impl Consumer {
    pub fn new(shared: Arc<Shared>, rate: u32) -> Self {
        Self {
            shared,
            epoch: 0,
            last: [0.0; 2],
            fade_remaining: 0,
            fade_length: (rate as usize / 200).max(1),
            fade_in: (rate as usize / 200).max(1),
            was_playing: false,
        }
    }

    pub fn next_frame(&mut self) -> [f32; 2] {
        let epoch = self.shared.epoch.load(Ordering::Acquire);
        let playing = self.shared.playing.load(Ordering::Acquire);
        if epoch != self.epoch || (self.was_playing && !playing) {
            self.fade_remaining = self.fade_length;
            self.fade_in = self.fade_length;
            self.epoch = epoch;
        }
        if !self.was_playing && playing {
            self.fade_in = self.fade_length;
        }
        self.was_playing = playing;
        if self.fade_remaining > 0 {
            self.fade_remaining -= 1;
            let gain = self.fade_remaining as f32 / self.fade_length as f32;
            return [self.last[0] * gain, self.last[1] * gain];
        }
        if !playing {
            return [0.0; 2];
        }
        while let Some(queued) = self.shared.queue.pop() {
            if queued.epoch != epoch {
                continue;
            }
            let mut frame = queued.frame.samples;
            if self.fade_in > 0 {
                let gain = 1.0 - self.fade_in as f32 / self.fade_length as f32;
                frame[0] *= gain;
                frame[1] *= gain;
                self.fade_in -= 1;
            }
            self.last = frame;
            self.shared
                .source_position
                .store(queued.frame.source_frame.to_bits(), Ordering::Relaxed);
            self.shared.output_frames.fetch_add(1, Ordering::Relaxed);
            return frame;
        }
        self.shared.underflow_frames.fetch_add(1, Ordering::Relaxed);
        // Ramp to silence on underrun, rather than making a hard step.
        self.fade_remaining = self.fade_length;
        self.fade_in = self.fade_length;
        self.last
    }
}

enum Command {
    Controls(Controls, SyncSender<()>),
    Seek(f64, Instant, SyncSender<()>),
    Loop(Option<LoopRegion>, SyncSender<()>),
    Shutdown,
}

pub struct Player {
    _stream: cpal::Stream,
    commands: SyncSender<Command>,
    worker: Option<JoinHandle<()>>,
    pub shared: Arc<Shared>,
    pub rate: u32,
    pub source_rate: u32,
    pub duration: f64,
    pub device_name: String,
    pub device_buffer_frames: u32,
}

impl Player {
    pub fn open(audio: Audio, finer: bool) -> Result<Self> {
        Self::open_shared(Arc::new(audio), finer)
    }

    pub fn open_shared(audio: Arc<Audio>, finer: bool) -> Result<Self> {
        let host = cpal::default_host();
        let device = host
            .default_output_device()
            .context("No audio output device")?;
        let device_name = device
            .name()
            .unwrap_or_else(|_| "Unknown output device".into());
        let source_rate = audio.rate;
        let supported = device
            .supported_output_configs()
            .context("Enumerating audio output configurations")?
            .find(|c| {
                c.channels() == 2
                    && c.sample_format() == cpal::SampleFormat::F32
                    && c.min_sample_rate().0 <= source_rate
                    && c.max_sample_rate().0 >= source_rate
            });
        let supported = match supported {
            Some(c) => c.with_sample_rate(cpal::SampleRate(source_rate)),
            None => device
                .default_output_config()
                .context("Reading default audio output configuration")?,
        };
        let rate = supported.sample_rate().0;
        let sample_format = supported.sample_format();
        let channels = supported.channels() as usize;
        let mut config: cpal::StreamConfig = supported.clone().into();
        // Small device buffers make control latency measurable; fall back to
        // the device's permitted range where a fixed size is advertised.
        let device_buffer_frames = match supported.buffer_size() {
            cpal::SupportedBufferSize::Range { min, max } => {
                let size = 256_u32.clamp(*min, *max);
                config.buffer_size = cpal::BufferSize::Fixed(size);
                size
            }
            cpal::SupportedBufferSize::Unknown => 0,
        };
        let duration = audio.duration();
        let audio = if rate == source_rate {
            audio
        } else {
            Arc::new(audio.resample(rate)?)
        };
        let shared = Arc::new(Shared::new((rate as usize / 10).max(1024)));
        let stream = match sample_format {
            cpal::SampleFormat::F32 => {
                make_stream::<f32>(&device, &config, channels, rate, shared.clone())
            }
            cpal::SampleFormat::I16 => {
                make_stream::<i16>(&device, &config, channels, rate, shared.clone())
            }
            cpal::SampleFormat::U16 => {
                make_stream::<u16>(&device, &config, channels, rate, shared.clone())
            }
            format => anyhow::bail!("Unsupported output format {format:?}"),
        }
        .context("Opening native audio output stream")?;
        let (commands, receiver) = mpsc::sync_channel(32);
        let worker_shared = shared.clone();
        let worker = thread::Builder::new()
            .name("ohmyloop-render".into())
            .spawn(move || {
                if let Err(error) = run_worker(audio, finer, receiver, &worker_shared) {
                    worker_shared.playing.store(false, Ordering::Release);
                    *worker_shared.worker_error.lock().unwrap() = Some(format!("{error:#}"));
                }
            })?;
        stream
            .play()
            .context("Starting native audio callback stream")?;
        Ok(Self {
            _stream: stream,
            commands,
            worker: Some(worker),
            shared,
            rate,
            source_rate,
            duration,
            device_name,
            device_buffer_frames,
        })
    }

    pub fn play(&self) -> Result<()> {
        ensure!(
            !self.shared.finished.load(Ordering::Acquire),
            "At end of song; seek before playing"
        );
        // Prime off the callback thread so startup isn't counted as an underrun.
        let deadline = Instant::now() + Duration::from_secs(2);
        while self.shared.queue.is_empty() && Instant::now() < deadline {
            ensure!(
                self.shared.worker_error.lock().unwrap().is_none(),
                "Audio worker failed"
            );
            thread::sleep(Duration::from_millis(1));
        }
        ensure!(
            !self.shared.queue.is_empty(),
            "No audio ready; seek before playing"
        );
        self.shared.playing.store(true, Ordering::Release);
        Ok(())
    }
    pub fn pause(&self) {
        self.shared.playing.store(false, Ordering::Release);
    }
    pub fn set_controls(&self, controls: Controls) -> Result<()> {
        let controls = controls.validate()?;
        self.request(|reply| Command::Controls(controls, reply))
    }
    pub fn seek(&self, seconds: f64) -> Result<()> {
        ensure!(
            seconds.is_finite() && (0.0..=self.duration).contains(&seconds),
            "Seek outside song"
        );
        self.request(|reply| Command::Seek(seconds, Instant::now(), reply))
    }
    pub fn set_loop(&self, region: Option<LoopRegion>) -> Result<()> {
        if let Some(region) = region {
            region.validate(self.duration)?;
        }
        self.request(|reply| Command::Loop(region, reply))
    }

    // Only the control/UI thread waits. The device callback never does.
    fn request(&self, command: impl FnOnce(SyncSender<()>) -> Command) -> Result<()> {
        let (reply, receiver) = mpsc::sync_channel(1);
        self.commands.send(command(reply))?;
        receiver
            .recv_timeout(Duration::from_secs(2))
            .context("Audio worker did not acknowledge the command")?;
        Ok(())
    }

    pub fn status(&self) -> String {
        let s = &self.shared;
        let position = s.position() / self.rate as f64;
        let input = f64::from_bits(s.render_position.load(Ordering::Relaxed));
        let message = format!(
            "{} | estimated output {:.3}/{:.3}s | input {:.3}s | queued {:.1}ms | underrun events {} | max render {:.2}ms | last seek setup {:.2}ms | input loop wraps {} | callbacks {} | device errors {}",
            if s.playing.load(Ordering::Relaxed) {
                "playing"
            } else {
                "paused"
            },
            position,
            self.duration,
            input,
            s.queue.len() as f64 * 1000.0 / self.rate as f64,
            s.underflow_frames.load(Ordering::Relaxed),
            s.max_render_us.load(Ordering::Relaxed) as f64 / 1000.0,
            s.last_seek_us.load(Ordering::Relaxed) as f64 / 1000.0,
            s.loop_wraps.load(Ordering::Relaxed),
            s.callbacks.load(Ordering::Relaxed),
            s.device_errors.load(Ordering::Relaxed)
        );
        if let Some(error) = s.worker_error.lock().unwrap().as_ref() {
            format!("{message}\nWORKER ERROR: {error}")
        } else {
            message
        }
    }
}

impl Drop for Player {
    fn drop(&mut self) {
        self.pause();
        let _ = self.commands.send(Command::Shutdown);
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

fn make_stream<T>(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    channels: usize,
    rate: u32,
    shared: Arc<Shared>,
) -> Result<cpal::Stream>
where
    T: cpal::SizedSample + cpal::FromSample<f32>,
{
    let mut consumer = Consumer::new(shared.clone(), rate);
    let errors = shared.clone();
    Ok(device.build_output_stream(
        config,
        move |output: &mut [T], _| {
            shared.callbacks.fetch_add(1, Ordering::Relaxed);
            for destination in output.chunks_exact_mut(channels) {
                let frame = consumer.next_frame();
                for (c, sample) in destination.iter_mut().enumerate() {
                    let value = if channels == 1 {
                        (frame[0] + frame[1]) * 0.5
                    } else if c < 2 {
                        frame[c]
                    } else {
                        0.0
                    };
                    *sample = T::from_sample(value.clamp(-1.0, 1.0));
                }
            }
        },
        move |_| {
            errors.device_errors.fetch_add(1, Ordering::Relaxed);
        },
        None,
    )?)
}

fn run_worker(
    audio: Arc<Audio>,
    finer: bool,
    receiver: Receiver<Command>,
    shared: &Shared,
) -> Result<()> {
    let mut controls = Controls::default();
    let mut region = None;
    let mut engine = Engine::new(audio.clone(), controls, 0.0, region, finer)?;
    let mut pending = Vec::new().into_iter();
    let mut ended = false;
    let target = audio.rate as usize / 20;
    loop {
        loop {
            let command = match receiver.try_recv() {
                Ok(command) => command,
                Err(mpsc::TryRecvError::Empty) => break,
                Err(mpsc::TryRecvError::Disconnected) => return Ok(()),
            };
            let reset = match command {
                Command::Shutdown => return Ok(()),
                Command::Controls(value, reply) => {
                    controls = value;
                    if shared.playing.load(Ordering::Acquire) {
                        engine.set_controls(controls)?;
                        let _ = reply.send(());
                        None
                    } else {
                        // Prebuffered audio must match controls chosen while paused.
                        Some((shared.position() / audio.rate as f64, None, reply))
                    }
                }
                Command::Seek(seconds, submitted, reply) => Some((seconds, Some(submitted), reply)),
                Command::Loop(value, reply) => {
                    region = value;
                    Some((shared.position() / audio.rate as f64, None, reply))
                }
            };
            if let Some((seconds, submitted, reply)) = reset {
                let new_engine = Engine::new(audio.clone(), controls, seconds, region, finer)?;
                shared.epoch.fetch_add(1, Ordering::AcqRel);
                while shared.queue.pop().is_some() {}
                pending = Vec::new().into_iter();
                shared.source_position.store(
                    (new_engine.output_position() * audio.rate as f64).to_bits(),
                    Ordering::Relaxed,
                );
                shared.finished.store(false, Ordering::Release);
                shared.loop_wraps.store(0, Ordering::Relaxed);
                engine = new_engine;
                ended = false;
                if let Some(submitted) = submitted {
                    shared
                        .last_seek_us
                        .store(submitted.elapsed().as_micros() as u64, Ordering::Relaxed);
                }
                // Capacity one, exactly one acknowledgement: no wait for the UI.
                let _ = reply.send(());
            }
        }
        if shared.queue.len() >= target {
            thread::sleep(Duration::from_millis(1));
            continue;
        }
        if pending.len() == 0 {
            if ended {
                if shared.queue.is_empty() {
                    shared.finished.store(true, Ordering::Release);
                    shared.playing.store(false, Ordering::Release);
                }
                thread::sleep(Duration::from_millis(1));
                continue;
            }
            let start = Instant::now();
            let block = engine.next_block();
            shared
                .max_render_us
                .fetch_max(start.elapsed().as_micros() as u64, Ordering::Relaxed);
            shared
                .render_position
                .store(engine.input_position().to_bits(), Ordering::Relaxed);
            shared
                .loop_wraps
                .store(engine.loop_wraps, Ordering::Relaxed);
            ended = block.is_empty();
            pending = block.into_iter();
        }
        while shared.queue.len() < target {
            let Some(frame) = pending.next() else {
                break;
            };
            // Only this worker pushes. The consumer can only make more room.
            shared
                .queue
                .push(QueuedFrame {
                    frame,
                    epoch: shared.epoch.load(Ordering::Acquire),
                })
                .map_err(|_| anyhow::anyhow!("Audio queue unexpectedly full"))?;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn wait_until(predicate: impl Fn() -> bool) {
        let deadline = Instant::now() + Duration::from_secs(3);
        while !predicate() {
            assert!(
                Instant::now() < deadline,
                "Audio worker did not reach expected state"
            );
            thread::sleep(Duration::from_millis(1));
        }
    }

    #[test]
    fn worker_prefills_pause_and_flushes_seek_and_loop_generations() {
        let audio = Arc::new(Audio::tone(48_000, 3.0, 440.0));
        let shared = Arc::new(Shared::new(4800));
        let (sender, receiver) = mpsc::sync_channel(32);
        let worker_shared = shared.clone();
        let worker = thread::spawn(move || run_worker(audio, true, receiver, &worker_shared));
        wait_until(|| shared.queue.len() == 2400);
        let mut consumer = Consumer::new(shared.clone(), 48_000);
        for _ in 0..512 {
            assert_eq!(consumer.next_frame(), [0.0; 2]);
        }
        assert_eq!(shared.queue.len(), 2400);
        let (reply, ack) = mpsc::sync_channel(1);
        sender
            .send(Command::Seek(1.25, Instant::now(), reply))
            .unwrap();
        ack.recv_timeout(Duration::from_secs(2)).unwrap();
        wait_until(|| shared.epoch.load(Ordering::Acquire) == 1 && shared.queue.len() == 2400);
        assert!((shared.position() / 48_000.0 - 1.25).abs() < 0.001);
        shared.playing.store(true, Ordering::Release);
        for _ in 0..512 {
            consumer.next_frame();
        }
        assert!(shared.position() / 48_000.0 >= 1.25);
        shared.playing.store(false, Ordering::Release);
        let region = LoopRegion {
            start: 0.5,
            end: 0.8,
            fade_ms: 5.0,
        };
        let (reply, ack) = mpsc::sync_channel(1);
        sender.send(Command::Loop(Some(region), reply)).unwrap();
        ack.recv_timeout(Duration::from_secs(2)).unwrap();
        wait_until(|| shared.epoch.load(Ordering::Acquire) == 2 && shared.queue.len() == 2400);
        assert!((shared.position() / 48_000.0 - 0.5).abs() < 0.001);
        shared.playing.store(true, Ordering::Release);
        for _ in 0..512 {
            consumer.next_frame();
        }
        assert!((0.5..0.8).contains(&(shared.position() / 48_000.0)));
        sender.send(Command::Shutdown).unwrap();
        worker.join().unwrap().unwrap();
    }
}
