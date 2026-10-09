//! Generated-audio hardware smoke check; no user song/settings are touched.
use anyhow::{Result, ensure};
use ohmyloop::{
    audio::Audio,
    dsp::Controls,
    engine::LoopRegion,
    metronome::ticks,
    playback::Player,
    timing::{Edit, MeasureMap, Meter},
};
use std::{
    sync::{Arc, atomic::Ordering},
    thread,
    time::Duration,
};
fn main() -> Result<()> {
    let rate = 48_000;
    let mut audio = Audio::tone(rate, 12.0, 220.0);
    for frame in &mut audio.frames {
        for sample in frame {
            *sample *= 0.15;
        }
    }
    let map = MeasureMap::empty(rate, audio.frames.len() as u64)
        .edit(Edit::Build {
            start: 0,
            end: rate as u64 * 12,
            count: 6,
            meter: Meter::common(),
        })?
        .map;
    let player = Player::open_shared(Arc::new(audio), true)?;
    player.set_metronome(true, 0.8, ticks(&map, player.rate)?)?;
    player.play()?;
    thread::sleep(Duration::from_secs(2));
    println!("normal playback: {}", player.status());
    player.set_controls(Controls {
        speed: 0.5,
        semitones: -5.0,
    })?;
    thread::sleep(Duration::from_secs(2));
    println!("slow / pitch shifted: {}", player.status());
    player.seek(4.0)?;
    player.set_loop(Some(LoopRegion {
        start: 4.0,
        end: 5.5,
        fade_ms: 5.0,
    }))?;
    player.set_controls(Controls {
        speed: 1.7,
        semitones: 7.0,
    })?;
    thread::sleep(Duration::from_secs(3));
    println!("seek / loop / fast: {}", player.status());
    let draft = map
        .edit(Edit::MoveBoundary {
            id: map.measures[2].end.clone(),
            frame: rate as u64 * 6 + rate as u64 / 3,
        })?
        .map;
    player.set_metronome(true, 0.4, ticks(&draft, player.rate)?)?;
    thread::sleep(Duration::from_secs(1));
    println!("live draft: {}", player.status());
    player.pause();
    player.set_metronome(false, 0.4, vec![])?;
    player.set_loop(None)?;
    player.seek(0.0)?;
    player.play()?;
    thread::sleep(Duration::from_secs(1));
    player.pause();
    println!(
        "{} ({} Hz)\n{}",
        player.device_name,
        player.rate,
        player.status()
    );
    ensure!(
        player.shared.callbacks.load(Ordering::Relaxed) > 0,
        "No callbacks"
    );
    ensure!(
        player.shared.device_errors.load(Ordering::Relaxed) == 0,
        "Device errors"
    );
    ensure!(
        player.shared.worker_error.lock().unwrap().is_none(),
        "Worker error"
    );
    ensure!(
        player.shared.underflow_frames.load(Ordering::Relaxed) == 0,
        "Underrun events"
    );
    println!(
        "PASS: authored clicks, speed/pitch, seek, loop, draft update, pause and disable executed. Listen separately for alignment/quality."
    );
    Ok(())
}
