// SPDX-License-Identifier: GPL-3.0-or-later
//! Listening harness for the production bounded source and shared practice DSP.
#[path = "stem_player/controls.rs"]
mod controls;
#[path = "stem_player/options.rs"]
mod options;
use anyhow::{Result, ensure};
use controls::Action;
use crossterm::{
    cursor::{Hide, MoveTo, Show},
    event::{self, Event},
    execute,
    terminal::{self, Clear, ClearType, EnterAlternateScreen, LeaveAlternateScreen},
};
use ohmyloop::{
    count_in::Settings, dsp::Controls, engine::LoopRegion, metronome, playback::Player,
};
use std::{
    io::{self, IsTerminal, Write},
    sync::atomic::Ordering,
    thread,
    time::Duration,
};

fn main() -> Result<()> {
    let Some(options) = options::Options::parse()? else {
        return Ok(());
    };
    ensure!(
        options.check || options.smoke || (io::stdin().is_terminal() && io::stdout().is_terminal()),
        "Run in an interactive terminal or use --check/--smoke"
    );
    let rate = if options.check {
        options.rate.unwrap_or(48_000)
    } else {
        let rate = Player::output_rate()?;
        ensure!(
            options.rate.is_none_or(|v| v == rate),
            "--rate must match the output device's {rate} Hz"
        );
        rate
    };
    // Preparation API supplied by the native bounded-source module.
    let prepared = prepare(&options, rate)?;
    if options.check {
        println!("Preparation validated; no audio device opened.");
        return Ok(());
    }
    let player = Player::open_bounded(prepared.clone(), true, prepared.reference_rate)?;
    let map = options.map(prepared.reference_rate, prepared.reference_frames)?;
    let mut state = State::new(prepared.source_count());
    if options.smoke {
        player.play()?;
        for action in [
            Action::Source,
            Action::Mute(4),
            Action::Speed(-0.5),
            Action::Pitch(3.0),
        ] {
            thread::sleep(Duration::from_secs(2));
            state.apply(action, &player, map.as_ref())?;
            check_error(&player)?;
        }
        player.pause();
        println!(
            "Hardware smoke: {}. Not a subjective listening verdict.",
            player.status()
        );
        return Ok(());
    }
    let terminal = Terminal::enter()?;
    loop {
        if let Err(error) = check_error(&player) {
            player.pause();
            drop(terminal);
            return Err(error);
        }
        draw(&player, &prepared, &state)?;
        if event::poll(Duration::from_millis(100))?
            && let Event::Key(key) = event::read()?
            && let Some(action) = controls::action(key)
        {
            if action == Action::Quit {
                break;
            }
            // Input validation errors remain visible; audio read failures stop via check_error.
            state.message = state
                .apply(action, &player, map.as_ref())
                .err()
                .map(|e| format!("{e:#}"));
        }
    }
    player.pause();
    Ok(())
}
fn check_error(player: &Player) -> Result<()> {
    if let Some(error) = player.shared.worker_error.lock().unwrap().as_ref() {
        anyhow::bail!("Playback stopped: {error}");
    }
    ensure!(
        player.shared.device_errors.load(Ordering::Relaxed) == 0,
        "Audio output device failed"
    );
    Ok(())
}
struct Terminal;
impl Terminal {
    fn enter() -> Result<Self> {
        terminal::enable_raw_mode()?;
        let guard = Self;
        execute!(io::stdout(), EnterAlternateScreen, Hide)?;
        Ok(guard)
    }
}
impl Drop for Terminal {
    fn drop(&mut self) {
        let _ = execute!(io::stdout(), Show, LeaveAlternateScreen);
        let _ = terminal::disable_raw_mode();
    }
}
struct State {
    selection: usize,
    source_count: usize,
    role: usize,
    mix: ohmyloop::stems::MixState,
    controls: Controls,
    loop_start: Option<f64>,
    region: Option<LoopRegion>,
    metronome: bool,
    count_in: bool,
    message: Option<String>,
}
impl State {
    fn new(source_count: usize) -> Self {
        Self {
            selection: 0,
            source_count,
            role: 0,
            mix: Default::default(),
            controls: Controls::default(),
            loop_start: None,
            region: None,
            metronome: false,
            count_in: false,
            message: None,
        }
    }
    fn apply(
        &mut self,
        action: Action,
        player: &Player,
        map: Option<&ohmyloop::timing::MeasureMap>,
    ) -> Result<()> {
        let position =
            (player.shared.position() / f64::from(player.rate)).clamp(0.0, player.duration);
        match action {
            Action::Mute(role) => {
                self.mix.tracks[role].mute = !self.mix.tracks[role].mute;
                player.set_mix(self.selection, self.mix)?;
            }
            Action::Solo => {
                self.mix.tracks[self.role].solo = !self.mix.tracks[self.role].solo;
                player.set_mix(self.selection, self.mix)?;
            }
            Action::Gain(delta) => {
                self.mix.tracks[self.role].gain =
                    (self.mix.tracks[self.role].gain + delta).clamp(0.0, 1.0);
                player.set_mix(self.selection, self.mix)?;
            }
            Action::Role(delta) => self.role = (self.role as i32 + delta).rem_euclid(6) as usize,
            Action::Source => {
                self.selection = (self.selection + 1) % self.source_count;
                player.set_mix(self.selection, self.mix)?;
            }
            Action::PlayPause => {
                if player.shared.playing.load(Ordering::Acquire) {
                    player.pause();
                } else {
                    if player.shared.finished.load(Ordering::Acquire) {
                        player.seek(0.0)?;
                    }
                    player.play()?;
                }
            }
            Action::Seek(delta) => player.seek((position + delta).clamp(0.0, player.duration))?,
            Action::LoopStart => self.loop_start = Some(position),
            Action::LoopEnd => {
                let start = self.loop_start.unwrap_or(0.0);
                let region = LoopRegion {
                    start,
                    end: position,
                    fade_ms: 5.0,
                }
                .validate(player.duration)?;
                player.set_loop(Some(region))?;
                self.region = Some(region);
            }
            Action::LoopClear => {
                player.set_loop(None)?;
                self.region = None;
                self.loop_start = None;
            }
            Action::Speed(delta) => {
                self.controls.speed = (self.controls.speed + delta).clamp(0.25, 2.0);
                player.set_controls(self.controls)?;
            }
            Action::Pitch(delta) => {
                self.controls.semitones = (self.controls.semitones + delta).clamp(-12.0, 12.0);
                player.set_controls(self.controls)?;
            }
            Action::Metronome => {
                let map =
                    map.ok_or_else(|| anyhow::anyhow!("Supply --bpm to enable the metronome"))?;
                self.metronome = !self.metronome;
                player.set_metronome(self.metronome, 0.25, metronome::ticks(map, player.rate)?)?;
            }
            Action::CountIn => {
                let map = map.ok_or_else(|| anyhow::anyhow!("Supply --bpm to enable count-in"))?;
                self.count_in = !self.count_in;
                player.set_count_in(
                    Settings {
                        enabled: self.count_in,
                        repeat: true,
                        volume: 0.25,
                    },
                    Some(map.clone()),
                )?;
            }
            Action::Quit => {}
        }
        Ok(())
    }
}

fn prepare(
    options: &options::Options,
    rate: u32,
) -> Result<std::sync::Arc<ohmyloop::stems::PreparedComparison>> {
    let prepared = ohmyloop::stems::PreparedComparison::prepare(
        &options.reference,
        &options.sets,
        rate,
        |p| {
            eprint!(
                "\r{}: {} / {}                    ",
                p.phase,
                p.completed,
                p.total.map_or_else(|| "?".into(), |v| v.to_string())
            );
        },
    )?;
    eprintln!(
        "\nPrepared {:.1}s at {} Hz in {:.2}s; temporary PCM {:.1} MiB; common attenuation {:.2} dB.",
        prepared.frames as f64 / f64::from(rate),
        rate,
        prepared.elapsed.as_secs_f64(),
        prepared.temp_bytes as f64 / 1048576.0,
        20.0 * prepared.shared_attenuation.log10()
    );
    Ok(std::sync::Arc::new(prepared))
}
fn draw(
    player: &Player,
    prepared: &ohmyloop::stems::PreparedComparison,
    state: &State,
) -> Result<()> {
    let mut stdout = io::stdout().lock();
    execute!(stdout, MoveTo(0, 0), Clear(ClearType::All))?;
    write!(
        stdout,
        "Stem practice player — shared Rubber Band R3\r\n\r\nSource: {} | {:.2}× | {:+.0} semitones\r\n{}\r\nCommon attenuation {:.2} dB | {} at {} Hz\r\n",
        ["Reference", "A", "B"][state.selection],
        state.controls.speed,
        state.controls.semitones,
        player.status(),
        20.0 * prepared.shared_attenuation.log10(),
        player.device_name,
        player.rate
    )?;
    let roles = prepared.source_roles(state.selection)?;
    for role in ohmyloop::stems::Role::ALL {
        let index = role.index();
        let track = state.mix.tracks[index];
        write!(
            stdout,
            "{}{} {:<7} {:3.0}% {} {} {}\r\n",
            if index == state.role { ">" } else { " " },
            index + 1,
            role.name(),
            track.gain * 100.0,
            if track.mute { "MUTED" } else { "     " },
            if track.solo { "SOLO" } else { "    " },
            if state.selection == 0 {
                "bypassed"
            } else if !roles.contains(&role) {
                "unavailable"
            } else {
                "available"
            }
        )?;
    }
    write!(
        stdout,
        "\r\nLoop {:?} | pending start {:?} | metronome {} | count-in {}\r\nPre-clamp peak {:.4} | clipped samples {}\r\n{}\r\n{}\r\n",
        state.region,
        state.loop_start,
        state.metronome,
        state.count_in,
        f64::from_bits(player.shared.pre_clamp_peak.load(Ordering::Relaxed)),
        player.shared.clipped_samples.load(Ordering::Relaxed),
        state.message.as_deref().unwrap_or(""),
        controls::help()
    )?;
    stdout.flush()?;
    Ok(())
}
