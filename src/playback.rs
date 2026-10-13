use crate::{
    audio::Audio,
    count_in::{CountIn, Plan as CountInPlan, Settings as CountInSettings},
    dsp::Controls,
    engine::MemorySource,
    engine::{Engine, LoopRegion, RenderedFrame},
    metronome::{ClickMixer, Tick},
    stems::{MixState, SourceFactory},
    timing::MeasureMap,
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
    pub count_in: bool,
}

pub struct Shared {
    pub queue: ArrayQueue<QueuedFrame>,
    pub epoch: AtomicU64,
    pub playing: AtomicBool,
    pub count_in: AtomicBool,
    preserve_queued_count_in: AtomicBool,
    pub finished: AtomicBool,
    pub source_position: AtomicU64,
    pub render_position: AtomicU64,
    pub underflow_frames: AtomicU64,
    pub callbacks: AtomicU64,
    pub pre_clamp_peak: AtomicU64,
    pub clipped_samples: AtomicU64,
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
            count_in: AtomicBool::new(false),
            preserve_queued_count_in: AtomicBool::new(false),
            finished: AtomicBool::new(false),
            source_position: AtomicU64::new(0_f64.to_bits()),
            render_position: AtomicU64::new(0_f64.to_bits()),
            underflow_frames: AtomicU64::new(0),
            callbacks: AtomicU64::new(0),
            pre_clamp_peak: AtomicU64::new(0_f64.to_bits()),
            clipped_samples: AtomicU64::new(0),
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
            // Mix edits during count-in invalidate only music, preserving the
            // queued click chronology without moving frames out of the queue.
            if !self.shared.preserve_queued_count_in.load(Ordering::Acquire) || !playing {
                self.fade_remaining = self.fade_length;
                self.fade_in = self.fade_length;
            }
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
            if queued.epoch != epoch
                && !(queued.count_in
                    && self.shared.preserve_queued_count_in.load(Ordering::Acquire))
            {
                continue;
            }
            let mut frame = queued.frame.samples;
            if self.fade_in > 0 {
                let gain = 1.0 - self.fade_in as f32 / self.fade_length as f32;
                frame[0] *= gain;
                frame[1] *= gain;
                self.fade_in -= 1;
            }
            for value in &mut frame {
                self.shared
                    .pre_clamp_peak
                    .fetch_max(f64::from(value.abs()).to_bits(), Ordering::Relaxed);
                if value.abs() > 1.0 {
                    self.shared.clipped_samples.fetch_add(1, Ordering::Relaxed);
                }
                *value = value.clamp(-1.0, 1.0);
            }
            self.last = frame;
            self.shared
                .source_position
                .store(queued.frame.source_frame.to_bits(), Ordering::Relaxed);
            self.shared
                .count_in
                .store(queued.count_in, Ordering::Relaxed);
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
    Metronome(bool, f32, Vec<Tick>, SyncSender<()>),
    CountIn(CountInSettings, Option<MeasureMap>, SyncSender<()>),
    Start(SyncSender<()>),
    Mix(usize, MixState, SyncSender<()>),
    Shutdown,
}

pub struct Player {
    _stream: Option<cpal::Stream>,
    commands: SyncSender<Command>,
    worker: Option<JoinHandle<()>>,
    pub shared: Arc<Shared>,
    pub rate: u32,
    pub source_rate: u32,
    pub source_frames: u64,
    pub duration: f64,
    pub device_name: String,
    pub device_buffer_frames: u32,
}

impl Player {
    pub fn open(audio: Audio, finer: bool) -> Result<Self> {
        Self::open_shared(Arc::new(audio), finer)
    }

    pub fn open_shared(audio: Arc<Audio>, finer: bool) -> Result<Self> {
        Self::open_factory(Arc::new(AudioFactory(audio)), finer, None)
    }

    pub fn open_bounded(
        source: Arc<dyn SourceFactory>,
        finer: bool,
        reference_rate: u32,
    ) -> Result<Self> {
        Self::open_factory(source, finer, Some(reference_rate))
    }

    pub fn headless(
        source: Arc<dyn SourceFactory>,
        finer: bool,
        reference_rate: u32,
    ) -> Result<Self> {
        let rate = source.rate();
        let (canonical_rate, source_frames) = source.reference_geometry();
        ensure!(
            reference_rate == canonical_rate,
            "Reference sample rate mismatch"
        );
        let duration = source.len_frames() as f64 / rate as f64;
        let shared = Arc::new(Shared::new((rate as usize / 10).max(1024)));
        let (commands, receiver) = mpsc::sync_channel(32);
        let worker_shared = shared.clone();
        let worker = thread::spawn(move || {
            if let Err(error) = run_source_worker(source, finer, receiver, &worker_shared) {
                worker_shared.playing.store(false, Ordering::Release);
                *worker_shared.worker_error.lock().unwrap() = Some(format!("{error:#}"));
            }
        });
        Ok(Self {
            _stream: None,
            commands,
            worker: Some(worker),
            shared,
            rate,
            source_rate: reference_rate,
            source_frames,
            duration,
            device_name: "software consumer".into(),
            device_buffer_frames: 0,
        })
    }

    pub fn output_rate() -> Result<u32> {
        Ok(cpal::default_host()
            .default_output_device()
            .context("No audio output device")?
            .default_output_config()?
            .sample_rate()
            .0)
    }

    fn open_factory(
        mut source: Arc<dyn SourceFactory>,
        finer: bool,
        reference_rate: Option<u32>,
    ) -> Result<Self> {
        let host = cpal::default_host();
        let device = host
            .default_output_device()
            .context("No audio output device")?;
        let device_name = device
            .name()
            .unwrap_or_else(|_| "Unknown output device".into());
        let (canonical_rate, source_frames) = source.reference_geometry();
        let source_rate = reference_rate.unwrap_or(canonical_rate);
        ensure!(
            source_rate == canonical_rate,
            "Reference sample rate mismatch"
        );
        let requested_rate = source.rate();
        let supported = device
            .supported_output_configs()
            .context("Enumerating audio output configurations")?
            .find(|c| {
                c.channels() == 2
                    && c.sample_format() == cpal::SampleFormat::F32
                    && c.min_sample_rate().0 <= requested_rate
                    && c.max_sample_rate().0 >= requested_rate
            });
        let supported = match supported {
            Some(c) => c.with_sample_rate(cpal::SampleRate(requested_rate)),
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
        let duration = source.len_frames() as f64 / source.rate() as f64;
        if rate != source.rate() {
            source = source.resample_legacy(rate)?;
        }
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
                if let Err(error) = run_source_worker(source, finer, receiver, &worker_shared) {
                    worker_shared.playing.store(false, Ordering::Release);
                    *worker_shared.worker_error.lock().unwrap() = Some(format!("{error:#}"));
                }
            })?;
        stream
            .play()
            .context("Starting native audio callback stream")?;
        Ok(Self {
            _stream: Some(stream),
            commands,
            worker: Some(worker),
            shared,
            rate,
            source_rate,
            source_frames,
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
        if self.shared.playing.load(Ordering::Acquire) {
            return Ok(());
        }
        self.request(Command::Start)?;
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
    pub fn set_mix(&self, selection: usize, mix: MixState) -> Result<()> {
        mix.validate()?;
        self.request(|reply| Command::Mix(selection, mix, reply))
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

    pub fn set_metronome(&self, enabled: bool, volume: f32, ticks: Vec<Tick>) -> Result<()> {
        ensure!(
            volume.is_finite() && (0.0..=1.0).contains(&volume),
            "Click volume must be between 0 and 1"
        );
        ensure!(
            ticks.len() <= 80_000
                && ticks.iter().all(|t| t.source_frame.is_finite()
                    && t.source_frame >= 0.0
                    && t.source_frame < self.duration * f64::from(self.rate))
                && ticks
                    .windows(2)
                    .all(|w| w[0].source_frame < w[1].source_frame),
            "Invalid metronome schedule"
        );
        self.request(|reply| Command::Metronome(enabled, volume, ticks, reply))
    }

    pub fn set_count_in(&self, settings: CountInSettings, map: Option<MeasureMap>) -> Result<()> {
        let settings = settings.validate()?;
        if let Some(map) = &map {
            map.validate()?;
            ensure!(
                map.rate == self.source_rate && map.frames == self.source_frames,
                "Count-in map belongs to a different source"
            );
            ensure!(
                !settings.enabled
                    || map
                        .measures
                        .iter()
                        .all(
                            |m| (m.meter.length().value() / m.meter.pulse.value()).ceil() <= 512.0
                        ),
                "Choose a larger pulse note value for the count-in"
            );
        }
        ensure!(
            !settings.enabled || map.as_ref().is_some_and(|m| !m.measures.is_empty()),
            "Build a measure map before enabling count-in"
        );
        self.request(|reply| Command::CountIn(settings, map, reply))
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
            "{} | estimated output {:.3}/{:.3}s | input {:.3}s | queued {:.1}ms | underrun events {} | max render {:.2}ms | last seek setup {:.2}ms | input loop wraps {} | callbacks {} | device errors {} | pre-clamp peak {:.4} | clipped samples {}",
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
            s.device_errors.load(Ordering::Relaxed),
            f64::from_bits(s.pre_clamp_peak.load(Ordering::Relaxed)),
            s.clipped_samples.load(Ordering::Relaxed)
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

#[cfg(test)]
fn run_worker(
    audio: Arc<Audio>,
    finer: bool,
    receiver: Receiver<Command>,
    shared: &Shared,
) -> Result<()> {
    run_source_worker(Arc::new(AudioFactory(audio)), finer, receiver, shared)
}

fn run_source_worker(
    source: Arc<dyn SourceFactory>,
    finer: bool,
    receiver: Receiver<Command>,
    shared: &Shared,
) -> Result<()> {
    let rate = source.rate();
    let mut selection = 0;
    let mut mix = MixState::default();
    let mut controls = Controls::default();
    let mut region = None;
    let mut engine =
        Engine::new_source(source.open(selection, mix)?, controls, 0.0, region, finer)?;
    let mut pending = Vec::new().into_iter();
    let mut ended = false;
    let mut metronome = ClickMixer::new(rate);
    let mut count_in_settings = CountInSettings::default();
    let mut count_in_map: Option<MeasureMap> = None;
    let mut count_in: Option<CountIn> = None;
    let mut repeat_plan: Option<CountInPlan> = None;
    let mut held_frame = None;
    let mut last_music_frame = None;
    let target = rate as usize / 20;
    loop {
        loop {
            let command = match receiver.try_recv() {
                Ok(command) => command,
                Err(mpsc::TryRecvError::Empty) => break,
                Err(mpsc::TryRecvError::Disconnected) => return Ok(()),
            };
            let mut start_count_in = false;
            let mut preserve_count_in = false;
            let reset = match command {
                Command::Shutdown => return Ok(()),
                Command::Mix(value, settings, reply) => {
                    selection = value;
                    mix = settings;
                    preserve_count_in = true;
                    if count_in.is_some() || shared.count_in.load(Ordering::Acquire) {
                        let position = count_in
                            .as_ref()
                            .map_or(shared.position(), |c| c.source_frame);
                        engine = Engine::new_source(
                            source.open(selection, mix)?,
                            controls,
                            position / rate as f64,
                            region,
                            finer,
                        )?;
                        shared
                            .preserve_queued_count_in
                            .store(true, Ordering::Release);
                        shared.epoch.fetch_add(1, Ordering::AcqRel);
                        pending = Vec::new().into_iter();
                        held_frame = None;
                        last_music_frame = None;
                        ended = false;
                        let _ = reply.send(());
                        None
                    } else {
                        Some((shared.position() / rate as f64, None, reply))
                    }
                }
                Command::Start(reply) => {
                    if count_in_settings.enabled {
                        start_count_in = true;
                        Some((shared.position() / rate as f64, None, reply))
                    } else if shared.count_in.load(Ordering::Acquire) || count_in.is_some() {
                        Some((shared.position() / rate as f64, None, reply))
                    } else {
                        let _ = reply.send(());
                        None
                    }
                }
                Command::CountIn(settings, map, reply) => {
                    count_in_settings = settings;
                    if let Some(active) = &mut count_in {
                        active.set_volume(settings.volume);
                    }
                    count_in_map = map;
                    repeat_plan = if settings.enabled && settings.repeat {
                        region
                            .map(|r: LoopRegion| {
                                CountInPlan::at(count_in_map.as_ref().unwrap(), r.start, rate)
                            })
                            .transpose()?
                    } else {
                        None
                    };
                    if !settings.enabled
                        && (count_in.is_some() || shared.count_in.load(Ordering::Acquire))
                    {
                        Some((shared.position() / rate as f64, None, reply))
                    } else {
                        let _ = reply.send(());
                        None
                    }
                }
                Command::Metronome(enabled, volume, ticks, reply) => {
                    metronome.configure(enabled, volume, ticks);
                    if shared.playing.load(Ordering::Acquire) {
                        let _ = reply.send(());
                        None
                    } else {
                        Some((shared.position() / rate as f64, None, reply))
                    }
                }
                Command::Controls(value, reply) => {
                    controls = value;
                    if shared.playing.load(Ordering::Acquire) {
                        if count_in.is_some() || shared.count_in.load(Ordering::Acquire) {
                            // Music primed behind a count-in must use the new
                            // controls too; retain the count-in's elapsed phase.
                            let source_position = count_in
                                .as_ref()
                                .map_or(shared.position(), |c| c.source_frame);
                            engine = Engine::new_source(
                                source.open(selection, mix)?,
                                controls,
                                source_position / f64::from(rate),
                                region,
                                finer,
                            )?;
                            pending = Vec::new().into_iter();
                            held_frame = None;
                            last_music_frame = None;
                            ended = false;
                            metronome.reset(region);
                        } else {
                            engine.set_controls(controls)?;
                        }
                        let _ = reply.send(());
                        None
                    } else {
                        // Prebuffered audio must match controls chosen while paused.
                        Some((shared.position() / rate as f64, None, reply))
                    }
                }
                Command::Seek(seconds, submitted, reply) => Some((seconds, Some(submitted), reply)),
                Command::Loop(value, reply) => {
                    region = value;
                    repeat_plan = if count_in_settings.enabled && count_in_settings.repeat {
                        region
                            .map(|r| CountInPlan::at(count_in_map.as_ref().unwrap(), r.start, rate))
                            .transpose()?
                    } else {
                        None
                    };
                    Some((shared.position() / rate as f64, None, reply))
                }
            };
            if let Some((seconds, submitted, reply)) = reset {
                let new_engine = Engine::new_source(
                    source.open(selection, mix)?,
                    controls,
                    seconds,
                    region,
                    finer,
                )?;
                shared
                    .preserve_queued_count_in
                    .store(false, Ordering::Release);
                shared.epoch.fetch_add(1, Ordering::AcqRel);
                while shared.queue.pop().is_some() {}
                pending = Vec::new().into_iter();
                shared.source_position.store(
                    (new_engine.output_position() * rate as f64).to_bits(),
                    Ordering::Relaxed,
                );
                shared.finished.store(false, Ordering::Release);
                shared.loop_wraps.store(0, Ordering::Relaxed);
                count_in = if preserve_count_in {
                    count_in
                } else if start_count_in {
                    Some(CountIn::new(
                        CountInPlan::at(
                            count_in_map.as_ref().unwrap(),
                            new_engine.output_position(),
                            rate,
                        )?
                        .with_silent_beat(),
                        rate,
                        count_in_settings.volume,
                        new_engine.output_position() * f64::from(rate),
                    ))
                } else {
                    None
                };
                shared.count_in.store(false, Ordering::Relaxed);
                held_frame = None;
                last_music_frame = None;
                engine = new_engine;
                metronome.reset(region);
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
        if pending.len() == 0 && held_frame.is_none() && !ended {
            let start = Instant::now();
            let block = engine.try_next_block()?;
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
        if ended && pending.len() == 0 && count_in.is_none() && held_frame.is_none() {
            if shared.queue.is_empty() {
                shared.finished.store(true, Ordering::Release);
                shared.playing.store(false, Ordering::Release);
            }
            thread::sleep(Duration::from_millis(1));
            continue;
        }
        while shared.queue.len() < target {
            let (frame, counting) = if let Some(active) = &mut count_in {
                if let Some(samples) = active.next(controls.speed) {
                    (
                        RenderedFrame {
                            samples,
                            source_frame: active.source_frame,
                        },
                        true,
                    )
                } else {
                    count_in = None;
                    continue;
                }
            } else {
                let Some(mut frame) = held_frame.take().or_else(|| pending.next()) else {
                    break;
                };
                let wrapped =
                    last_music_frame.is_some_and(|previous| frame.source_frame < previous);
                last_music_frame = Some(frame.source_frame);
                if wrapped && let Some(plan) = &repeat_plan {
                    count_in = Some(CountIn::new(
                        plan.clone(),
                        rate,
                        count_in_settings.volume,
                        region.map_or(frame.source_frame, |r| (r.start * f64::from(rate)).round()),
                    ));
                    held_frame = Some(frame);
                    metronome.reset(region);
                    continue;
                }
                frame.samples = metronome.mix(frame.source_frame, frame.samples);
                if repeat_plan.is_some()
                    && let Some(r) = region
                {
                    // Rubber Band may retain energy beyond the input fades.
                    // Pair output fades at the joins with inserted count-ins.
                    let start = (r.start * f64::from(rate)).round();
                    let end = (r.end * f64::from(rate)).round();
                    let distance = (frame.source_frame - start)
                        .min(end - frame.source_frame)
                        .max(0.0);
                    let gain =
                        (distance / (f64::from(rate) * controls.speed * 0.005)).min(1.0) as f32;
                    frame.samples.iter_mut().for_each(|sample| *sample *= gain);
                }
                (frame, false)
            };
            // Only this worker pushes. The consumer can only make more room.
            shared
                .queue
                .push(QueuedFrame {
                    frame,
                    count_in: counting,
                    epoch: shared.epoch.load(Ordering::Acquire),
                })
                .map_err(|_| anyhow::anyhow!("Audio queue unexpectedly full"))?;
        }
    }
}

struct AudioFactory(Arc<Audio>);
impl SourceFactory for AudioFactory {
    fn rate(&self) -> u32 {
        self.0.rate
    }
    fn len_frames(&self) -> u64 {
        self.0.frames.len() as u64
    }
    fn open(&self, _: usize, _: MixState) -> Result<Box<dyn crate::stems::BlockSource>> {
        Ok(Box::new(MemorySource(self.0.clone())))
    }
    fn resample_legacy(&self, rate: u32) -> Result<Arc<dyn SourceFactory>> {
        Ok(Arc::new(Self(Arc::new(self.0.resample(rate)?))))
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

    struct TestWorker {
        shared: Arc<Shared>,
        sender: SyncSender<Command>,
        worker: Option<JoinHandle<Result<()>>>,
    }
    impl TestWorker {
        fn new() -> Self {
            let audio = Arc::new(Audio::tone(48_000, 3.0, 440.0));
            let shared = Arc::new(Shared::new(4800));
            let (sender, receiver) = mpsc::sync_channel(32);
            let worker_shared = shared.clone();
            let worker = Some(thread::spawn(move || {
                run_worker(audio, true, receiver, &worker_shared)
            }));
            let result = Self {
                shared,
                sender,
                worker,
            };
            wait_until(|| result.shared.queue.len() == 2400);
            result
        }
        fn send(&self, command: impl FnOnce(SyncSender<()>) -> Command) {
            let (reply, ack) = mpsc::sync_channel(1);
            self.sender.send(command(reply)).unwrap();
            ack.recv_timeout(Duration::from_secs(2)).unwrap();
        }
        fn read(&self, count: usize) -> Vec<QueuedFrame> {
            let mut frames = Vec::with_capacity(count);
            while frames.len() < count {
                wait_until(|| !self.shared.queue.is_empty());
                while frames.len() < count {
                    let Some(frame) = self.shared.queue.pop() else {
                        break;
                    };
                    self.shared
                        .source_position
                        .store(frame.frame.source_frame.to_bits(), Ordering::Relaxed);
                    self.shared
                        .count_in
                        .store(frame.count_in, Ordering::Relaxed);
                    frames.push(frame);
                }
            }
            frames
        }
        fn configure(&self, speed: f64, semitones: f64, repeat: bool) {
            self.send(|reply| Command::Controls(Controls { speed, semitones }, reply));
            self.send(|reply| {
                Command::Loop(
                    Some(LoopRegion {
                        start: 0.5,
                        end: 1.0,
                        fade_ms: 5.0,
                    }),
                    reply,
                )
            });
            self.send(|reply| {
                Command::CountIn(
                    CountInSettings {
                        enabled: true,
                        repeat,
                        volume: 1.0,
                    },
                    worker_map(),
                    reply,
                )
            });
            self.send(Command::Start);
            self.shared.playing.store(true, Ordering::Release);
        }
    }
    impl Drop for TestWorker {
        fn drop(&mut self) {
            self.sender.send(Command::Shutdown).unwrap();
            self.worker.take().unwrap().join().unwrap().unwrap();
        }
    }

    #[test]
    fn count_in_output_is_separate_from_dsp_and_repeats_only_when_requested() {
        for repeat in [false, true] {
            for speed in [0.5, 1.0, 2.0] {
                let mut previous = None;
                for semitones in [-7.0, 5.0] {
                    let worker = TestWorker::new();
                    worker.configure(speed, semitones, repeat);
                    let length = (24_000.0 / speed) as usize;
                    let lead = (6000.0 / speed) as usize;
                    let intro = worker.read(length + lead);
                    assert!(
                        intro
                            .iter()
                            .all(|f| f.count_in && f.frame.source_frame == 24_000.0)
                    );
                    let samples: Vec<_> = intro.iter().map(|f| f.frame.samples).collect();
                    assert!(
                        samples[..lead].iter().all(|f| *f == [0.0; 2]),
                        "Manual starts must wait one silent beat"
                    );
                    assert!(
                        samples[lead..lead + 1000].iter().any(|f| f[0].abs() > 0.01),
                        "First click must follow the silent beat"
                    );
                    assert!(
                        samples.iter().any(|f| f[0].abs() > 0.01),
                        "Clicks must sound with metronome off"
                    );
                    if let Some(previous) = previous {
                        assert_eq!(
                            samples, previous,
                            "Pitch processing cannot alter count-in clicks"
                        );
                    }
                    previous = Some(samples);
                    let music = worker.read(length);
                    assert!(music.iter().all(
                        |f| !f.count_in && (24_000.0..48_000.0).contains(&f.frame.source_frame)
                    ));
                    let next = worker.read(length);
                    assert!(next.iter().all(|f| f.count_in == repeat));
                    if repeat {
                        assert!(next.iter().all(|f| f.frame.source_frame == 24_000.0));
                        assert!(
                            next[..1000].iter().any(|f| f.frame.samples[0].abs() > 0.01),
                            "Loop repetition clicks must start without the extra silent beat"
                        );
                    }
                    let next_music = worker.read(length);
                    assert!(next_music.iter().all(|f| !f.count_in));
                }
            }
        }
    }

    #[test]
    fn live_speed_changes_retime_the_remaining_count_in_and_primed_music() {
        let worker = TestWorker::new();
        worker.configure(1.0, 0.0, true);
        worker.read(6000);
        wait_until(|| worker.shared.queue.len() == 2400);
        worker.send(|reply| {
            Command::Controls(
                Controls {
                    speed: 0.5,
                    semitones: 7.0,
                },
                reply,
            )
        });
        let remaining = worker.read(2400 + (30_000 - 8400) * 2);
        assert!(remaining.iter().all(|f| f.count_in));
        let music = worker.read(24_000 * 2);
        assert!(music.iter().all(|f| !f.count_in));
        assert_eq!(music[0].frame.source_frame, 24_000.0);
        assert_eq!(music[1].frame.source_frame, 24_000.5);
        // Changing repetition intent live affects the next wrap, independently
        // of the initial count-in and of the metronome's enable state.
        worker.send(|reply| {
            Command::CountIn(
                CountInSettings {
                    enabled: true,
                    repeat: false,
                    volume: 0.8,
                },
                worker_map(),
                reply,
            )
        });
        // Up to 50 ms of the already queued repetition may remain. A seek
        // flushes it, then subsequent wraps must be uninterrupted music.
        worker.send(|reply| Command::Seek(0.5, Instant::now(), reply));
        assert!(worker.read(24_000 * 4).iter().all(|f| !f.count_in));
    }

    #[test]
    fn live_speed_changes_also_retime_the_silent_beat() {
        let worker = TestWorker::new();
        worker.configure(1.0, 0.0, false);
        let first = worker.read(1200);
        assert!(
            first
                .iter()
                .all(|f| f.count_in && f.frame.samples == [0.0; 2])
        );
        wait_until(|| worker.shared.queue.len() == 2400);
        worker.send(|reply| {
            Command::Controls(
                Controls {
                    speed: 0.5,
                    semitones: 0.0,
                },
                reply,
            )
        });
        let silence = worker.read(2400 + (6000 - 3600) * 2);
        assert!(
            silence
                .iter()
                .all(|f| f.count_in && f.frame.samples == [0.0; 2])
        );
        let clicks = worker.read(1000);
        assert!(
            clicks
                .iter()
                .all(|f| f.count_in && f.frame.source_frame == 24_000.0)
        );
        assert!(clicks.iter().any(|f| f.frame.samples[0].abs() > 0.01));
    }

    #[test]
    fn mix_commands_preserve_count_in_elapsed_phase_and_click_schedule() {
        for speed in [0.25, 1.0, 2.0] {
            let worker = TestWorker::new();
            worker.configure(speed, 0.0, false);
            let consumed = 1200;
            worker.read(consumed);
            wait_until(|| worker.shared.queue.len() == 2400);
            let mut mix = MixState::default();
            mix.tracks[0].mute = true;
            worker.send(|reply| Command::Mix(1, mix, reply));
            let remaining = (30_000.0 / speed) as usize - consumed;
            let frames = worker.read(remaining);
            assert!(
                frames.iter().all(|f| f.count_in),
                "Mix edits cannot skip count-in frames"
            );
            let music = worker.read(1200);
            assert!(music.iter().all(|f| !f.count_in));
            assert_eq!(music[0].frame.source_frame, 24_000.0);
        }
    }

    #[test]
    fn concurrent_mix_edits_do_not_reorder_or_fade_count_in_clicks() {
        fn capture(edits: bool) -> Vec<[f32; 2]> {
            let worker = TestWorker::new();
            worker.configure(1.0, 0.0, false);
            let shared = worker.shared.clone();
            let consumer_thread = thread::spawn(move || {
                let mut consumer = Consumer::new(shared.clone(), 48_000);
                let mut samples = Vec::new();
                let start = Instant::now();
                for block in 0..400 {
                    for _ in 0..48 {
                        samples.push(consumer.next_frame());
                    }
                    let due = start + Duration::from_millis(block + 1);
                    if let Some(remaining) = due.checked_duration_since(Instant::now()) {
                        thread::sleep(remaining);
                    }
                }
                assert_eq!(shared.underflow_frames.load(Ordering::Relaxed), 0);
                samples
            });
            if edits {
                for selection in [1, 2, 0, 1, 0] {
                    thread::sleep(Duration::from_millis(35));
                    worker.send(|reply| Command::Mix(selection, MixState::default(), reply));
                }
            }
            consumer_thread.join().unwrap()
        }
        assert_eq!(capture(false), capture(true));
    }

    fn worker_map() -> Option<MeasureMap> {
        use crate::timing::{Edit, Meter};
        Some(
            MeasureMap::empty(48_000, 144_000)
                .edit(Edit::Build {
                    start: 0,
                    end: 144_000,
                    count: 6,
                    meter: Meter::common(),
                })
                .unwrap()
                .map,
        )
    }

    #[test]
    fn seek_pause_restart_and_disabling_flush_count_in_frames() {
        let worker = TestWorker::new();
        worker.configure(1.0, 0.0, true);
        worker.read(1200);
        let mut consumer = Consumer::new(worker.shared.clone(), 48_000);
        consumer.next_frame();
        worker.shared.playing.store(false, Ordering::Release);
        for _ in 0..512 {
            consumer.next_frame();
        }
        let paused_at = worker.shared.position();
        let queued = worker.shared.queue.len();
        assert_eq!(consumer.next_frame(), [0.0; 2]);
        assert_eq!(worker.shared.position(), paused_at);
        assert!(worker.shared.queue.len() >= queued);
        worker.send(Command::Start);
        let intro = worker.read(30_000);
        assert!(
            intro
                .iter()
                .all(|f| f.count_in && f.frame.source_frame == 24_000.0)
        );
        worker.send(|reply| Command::Seek(0.75, Instant::now(), reply));
        assert!(
            worker
                .read(1200)
                .iter()
                .all(|f| !f.count_in && f.frame.source_frame >= 36_000.0)
        );
        worker.send(Command::Start);
        assert!(worker.read(1200).iter().all(|f| f.count_in));
        worker.send(|reply| Command::CountIn(CountInSettings::default(), None, reply));
        assert!(worker.read(1200).iter().all(|f| !f.count_in));
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

    #[test]
    fn paused_metronome_changes_replace_prebuffered_audio_and_seek_clears_click_history() {
        let rate = 48_000;
        let audio = Arc::new(Audio {
            rate,
            frames: vec![[0.0; 2]; rate as usize],
        });
        let shared = Arc::new(Shared::new(4800));
        let (sender, receiver) = mpsc::sync_channel(32);
        let worker_shared = shared.clone();
        let worker = thread::spawn(move || run_worker(audio, true, receiver, &worker_shared));
        wait_until(|| shared.queue.len() == 2400);
        let (reply, ack) = mpsc::sync_channel(1);
        sender
            .send(Command::Metronome(
                true,
                1.0,
                vec![Tick {
                    source_frame: 0.0,
                    accent: crate::timing::Accent::Measure,
                }],
                reply,
            ))
            .unwrap();
        ack.recv_timeout(Duration::from_secs(2)).unwrap();
        wait_until(|| shared.epoch.load(Ordering::Acquire) == 1 && shared.queue.len() == 2400);
        let mut consumer = Consumer::new(shared.clone(), rate);
        shared.playing.store(true, Ordering::Release);
        let samples: Vec<_> = (0..1200).map(|_| consumer.next_frame()[0]).collect();
        assert!(
            samples.iter().any(|s| s.abs() > 0.01),
            "First beat must survive callback startup fade"
        );
        shared.playing.store(false, Ordering::Release);
        let (reply, ack) = mpsc::sync_channel(1);
        sender
            .send(Command::Seek(0.5, Instant::now(), reply))
            .unwrap();
        ack.recv_timeout(Duration::from_secs(2)).unwrap();
        wait_until(|| shared.epoch.load(Ordering::Acquire) == 2 && shared.queue.len() == 2400);
        assert!((0..2400).all(|_| shared.queue.pop().unwrap().frame.samples == [0.0; 2]));
        sender.send(Command::Shutdown).unwrap();
        worker.join().unwrap().unwrap();
    }
}
