// SPDX-License-Identifier: GPL-3.0-or-later
use super::model::Banks;
use anyhow::{Context, Result, ensure};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use ohmyloop::{
    engine::RenderedFrame,
    playback::{Consumer, QueuedFrame, Shared},
};
use std::{
    sync::{
        Arc,
        atomic::Ordering,
        mpsc::{self, Receiver, SyncSender},
    },
    thread::{self, JoinHandle},
    time::Duration,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    ToggleStem(usize),
    Switch,
    PlayPause,
    Seek(i32),
    Quit,
}

#[derive(Default, Clone, Copy, Debug)]
pub struct Selection {
    pub bank: usize,
    pub muted: u8,
}
impl Selection {
    pub fn apply(&mut self, action: Action) {
        match action {
            Action::ToggleStem(index) => self.muted ^= 1 << index,
            Action::Switch => self.bank ^= 1,
            _ => (),
        }
    }
}

struct Command {
    action: Action,
    reply: SyncSender<()>,
}
pub struct Output {
    // Drop pauses consumption and joins the worker before fields are released.
    _stream: cpal::Stream,
    sender: SyncSender<Command>,
    worker: Option<JoinHandle<()>>,
    pub shared: Arc<Shared>,
    pub source_rate: u32,
    pub duration: f64,
    pub labels: [String; 2],
    pub gain: f32,
    pub device: String,
}
impl Output {
    pub fn open(mut banks: Banks) -> Result<Self> {
        let device = cpal::default_host()
            .default_output_device()
            .context("No output device")?;
        let name = device.name().unwrap_or_else(|_| "default output".into());
        let supported = device.supported_output_configs()?.find(|c| {
            c.channels() == 2
                && c.sample_format() == cpal::SampleFormat::F32
                && c.min_sample_rate().0 <= banks.source_rate
                && c.max_sample_rate().0 >= banks.source_rate
        });
        let supported = match supported {
            Some(c) => c.with_sample_rate(cpal::SampleRate(banks.source_rate)),
            None => device.default_output_config()?,
        };
        let rate = supported.sample_rate().0;
        let format = supported.sample_format();
        let mut config: cpal::StreamConfig = supported.clone().into();
        if let cpal::SupportedBufferSize::Range { min, max } = supported.buffer_size() {
            config.buffer_size = cpal::BufferSize::Fixed(256_u32.clamp(*min, *max));
        }
        eprintln!("Preparing {rate} Hz playback on {name}...");
        banks.resample(rate)?;
        let shared = Arc::new(Shared::new((rate as usize / 20).max(1024)));
        let stream = match format {
            cpal::SampleFormat::F32 => make_stream::<f32>(&device, &config, shared.clone()),
            cpal::SampleFormat::I16 => make_stream::<i16>(&device, &config, shared.clone()),
            cpal::SampleFormat::U16 => make_stream::<u16>(&device, &config, shared.clone()),
            _ => anyhow::bail!("Unsupported output sample format {format:?}"),
        }?;
        let (sender, receiver) = mpsc::sync_channel(16);
        let worker_shared = shared.clone();
        let source_rate = banks.source_rate;
        let duration = banks.duration();
        let labels = banks.labels.clone();
        let gain = banks.gain;
        let worker = thread::Builder::new()
            .name("stem-mixer-render".into())
            .spawn(move || run_worker(banks, receiver, worker_shared))?;
        // Construct the owner before starting, so startup failure joins the worker.
        let result = Self {
            _stream: stream,
            sender,
            worker: Some(worker),
            shared,
            source_rate,
            duration,
            labels,
            gain,
            device: name,
        };
        result._stream.play()?;
        Ok(result)
    }
    pub fn action(&self, action: Action) -> Result<()> {
        let (reply, receiver) = mpsc::sync_channel(1);
        self.sender.send(Command { action, reply })?;
        receiver
            .recv_timeout(Duration::from_secs(2))
            .context("Mixer worker did not acknowledge input")?;
        Ok(())
    }
    pub fn position(&self) -> f64 {
        self.shared.position() / f64::from(self.source_rate)
    }
    pub fn playing(&self) -> bool {
        self.shared.playing.load(Ordering::Acquire)
    }
    pub fn check_device(&self) -> Result<()> {
        ensure!(
            self.shared.device_errors.load(Ordering::Relaxed) == 0,
            "Audio device reported an error"
        );
        Ok(())
    }
}
impl Drop for Output {
    fn drop(&mut self) {
        self.shared.playing.store(false, Ordering::Release);
        let (reply, _) = mpsc::sync_channel(1);
        let _ = self.sender.send(Command {
            action: Action::Quit,
            reply,
        });
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

fn make_stream<T: cpal::SizedSample + cpal::FromSample<f32>>(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    shared: Arc<Shared>,
) -> Result<cpal::Stream> {
    let mut consumer = Consumer::new(shared.clone(), config.sample_rate.0);
    let channels = usize::from(config.channels);
    let errors = shared.clone();
    Ok(device.build_output_stream(
        config,
        move |output: &mut [T], _| {
            shared.callbacks.fetch_add(1, Ordering::Relaxed);
            for destination in output.chunks_exact_mut(channels) {
                let frame = consumer.next_frame();
                for (index, sample) in destination.iter_mut().enumerate() {
                    let value = if channels == 1 {
                        (frame[0] + frame[1]) * 0.5
                    } else if index < 2 {
                        frame[index]
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

fn run_worker(banks: Banks, receiver: Receiver<Command>, shared: Arc<Shared>) {
    let mut selection = Selection::default();
    let mut cursor = 0_usize;
    let rate = banks.rate();
    loop {
        while let Ok(command) = receiver.try_recv() {
            if command.action == Action::Quit {
                let _ = command.reply.send(());
                return;
            }
            match command.action {
                Action::PlayPause => {
                    let playing = shared.playing.load(Ordering::Acquire);
                    if !playing && cursor == banks.frames() && shared.queue.is_empty() {
                        cursor = 0;
                        shared
                            .source_position
                            .store(0_f64.to_bits(), Ordering::Relaxed);
                        shared.finished.store(false, Ordering::Release);
                    }
                    if !playing {
                        fill_queue(&banks, selection, &mut cursor, &shared);
                    }
                    shared.playing.store(!playing, Ordering::Release);
                }
                action => {
                    // Freeze consumption while replacing queued audio. Device-submitted
                    // audio cannot be recalled; Consumer pairs a 5 ms fade out/in.
                    let playing = shared.playing.swap(false, Ordering::AcqRel);
                    let source_position = shared.position();
                    let seconds = source_position / f64::from(banks.source_rate);
                    let seconds = if let Action::Seek(delta) = action {
                        (seconds + f64::from(delta)).clamp(0.0, banks.duration())
                    } else {
                        seconds
                    };
                    selection.apply(action);
                    cursor = (seconds * f64::from(rate))
                        .round()
                        .min(banks.frames() as f64) as usize;
                    shared.epoch.fetch_add(1, Ordering::AcqRel);
                    while shared.queue.pop().is_some() {}
                    shared.source_position.store(
                        (seconds * f64::from(banks.source_rate)).to_bits(),
                        Ordering::Relaxed,
                    );
                    shared.finished.store(false, Ordering::Release);
                    fill_queue(&banks, selection, &mut cursor, &shared);
                    shared
                        .playing
                        .store(playing && !shared.queue.is_empty(), Ordering::Release);
                }
            }
            let _ = command.reply.send(());
        }
        fill_queue(&banks, selection, &mut cursor, &shared);
        if cursor == banks.frames() && shared.queue.is_empty() {
            shared.finished.store(true, Ordering::Release);
            shared.playing.store(false, Ordering::Release);
        }
        thread::sleep(Duration::from_millis(1));
    }
}

fn fill_queue(banks: &Banks, selection: Selection, cursor: &mut usize, shared: &Shared) {
    while shared.queue.len() < shared.queue.capacity() && *cursor < banks.frames() {
        let source_frame = ((*cursor + 1) as f64 * f64::from(banks.source_rate)
            / f64::from(banks.rate()))
        .min(banks.source_frames as f64);
        let queued = QueuedFrame {
            frame: RenderedFrame {
                samples: banks.mix(selection.bank, selection.muted, *cursor),
                source_frame,
            },
            epoch: shared.epoch.load(Ordering::Acquire),
            count_in: false,
        };
        if shared.queue.push(queued).is_err() {
            break;
        }
        *cursor += 1;
    }
    shared.render_position.store(
        (*cursor as f64 * f64::from(banks.source_rate) / f64::from(banks.rate())).to_bits(),
        Ordering::Relaxed,
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    fn test_banks() -> Banks {
        use ohmyloop::audio::Audio;
        Banks {
            audio: [
                vec![
                    Audio {
                        rate: 1000,
                        frames: vec![[0.02; 2]; 10_000]
                    };
                    6
                ],
                vec![
                    Audio {
                        rate: 1000,
                        frames: vec![[0.04; 2]; 10_000]
                    };
                    6
                ],
            ],
            labels: ["A".into(), "B".into()],
            source_rate: 1000,
            source_frames: 10_000,
            gain: 1.0,
            provenance_checked: false,
        }
    }
    #[test]
    fn resampled_queue_positions_stay_on_original_stem_timeline() {
        let mut banks = test_banks();
        for bank in &mut banks.audio {
            for stem in bank {
                stem.rate = 2000;
                stem.frames.resize(20_000, [0.02; 2]);
            }
        }
        let shared = Shared::new(4);
        let mut cursor = 10_000;
        fill_queue(&banks, Selection::default(), &mut cursor, &shared);
        assert_eq!(shared.queue.pop().unwrap().frame.source_frame, 5000.5);
        assert_eq!(shared.queue.pop().unwrap().frame.source_frame, 5001.0);
        assert_eq!(
            f64::from_bits(shared.render_position.load(Ordering::Relaxed)),
            5002.0
        );
    }
    #[test]
    fn worker_switches_at_consumed_position_keeps_shared_mask_and_clamps_seek() {
        let shared = Arc::new(Shared::new(50));
        let (sender, receiver) = mpsc::sync_channel(16);
        let worker_shared = shared.clone();
        let worker = thread::spawn(move || run_worker(test_banks(), receiver, worker_shared));
        let action = |action| {
            let (reply, ack) = mpsc::sync_channel(1);
            sender.send(Command { action, reply }).unwrap();
            ack.recv_timeout(Duration::from_secs(2)).unwrap();
        };
        action(Action::Seek(5));
        action(Action::ToggleStem(2));
        action(Action::Switch);
        assert_eq!(shared.position(), 5000.0);
        let frame = shared.queue.pop().unwrap().frame;
        assert_eq!(frame.source_frame, 5001.0);
        assert!((frame.samples[0] - 0.2).abs() < 1e-6);
        action(Action::PlayPause);
        let mut consumer = Consumer::new(shared.clone(), 1000);
        for _ in 0..20 {
            consumer.next_frame();
        }
        let position = shared.position();
        action(Action::Switch);
        let frame = shared.queue.pop().unwrap().frame;
        assert_eq!(frame.source_frame, position + 1.0);
        assert!((frame.samples[0] - 0.1).abs() < 1e-6);
        action(Action::Seek(-50));
        assert_eq!(shared.position(), 0.0);
        action(Action::Seek(50));
        assert_eq!(shared.position(), 10_000.0);
        assert!(!shared.playing.load(Ordering::Acquire));
        action(Action::PlayPause);
        assert!(shared.playing.load(Ordering::Acquire));
        action(Action::Quit);
        worker.join().unwrap();
    }
}
