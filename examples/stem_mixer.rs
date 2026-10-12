// SPDX-License-Identifier: GPL-3.0-or-later
//! Standalone synchronized six-stem A/B listening tool; no desktop integration.
#[path = "stem_mixer/model.rs"]
mod model;
#[path = "stem_mixer/output.rs"]
mod output;

use anyhow::{Context, Result, ensure};
use crossterm::{
    cursor::{Hide, MoveTo, Show},
    event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers},
    execute,
    terminal::{self, Clear, ClearType, EnterAlternateScreen, LeaveAlternateScreen},
};
use model::{Banks, STEMS};
use output::{Action, Output, Selection};
use std::{
    io::{self, IsTerminal, Write},
    path::PathBuf,
    sync::atomic::Ordering,
    thread,
    time::Duration,
};

const HELP: &str = "1-6 mute/unmute | 0 A/B | Space play/pause | Left/Right seek 5s | Esc quit";

fn main() -> Result<()> {
    let mut mode = "interactive";
    let mut paths = Vec::new();
    for arg in std::env::args().skip(1) {
        match arg.as_str() {
            "--help" | "-h" => {
                println!(
                    "Six-stem A/B mixer\n\ncargo run --locked --release --example stem_mixer -- [DIR_A DIR_B] [--check | --smoke]\n\nDefaults: artifacts/stem-prototype/bones-demucs-6s and bones-roformer-sw\n{HELP}\n\n--check: load/validate PCM without opening an audio device\n--smoke: 4-second hardware playback, including switch/mute/seek; no terminal needed\n\nBoth banks require bass.wav, drums.wav, guitar.wav, piano.wav, vocals.wav, other.wav.\nStarts paused with all stems on; Space restarts from beginning at end."
                );
                return Ok(());
            }
            "--check" => {
                ensure!(mode == "interactive", "Choose one mode");
                mode = "check";
            }
            "--smoke" => {
                ensure!(mode == "interactive", "Choose one mode");
                mode = "smoke";
            }
            _ if arg.starts_with('-') => anyhow::bail!("Unknown option {arg}; use --help"),
            _ => paths.push(PathBuf::from(arg)),
        }
    }
    ensure!(
        paths.is_empty() || paths.len() == 2,
        "Provide both output directories, or neither; use --help"
    );
    if paths.is_empty() {
        paths = vec![
            "artifacts/stem-prototype/bones-demucs-6s".into(),
            "artifacts/stem-prototype/bones-roformer-sw".into(),
        ];
    }
    if mode == "interactive" {
        ensure!(
            io::stdin().is_terminal() && io::stdout().is_terminal(),
            "Run in an interactive terminal, or use --check / --smoke"
        );
    }
    let banks = Banks::load([&paths[0], &paths[1]])?;
    eprintln!(
        "Loaded {:.3}s at {} Hz. Common gain {:.6} ({:.1} dB).",
        banks.duration(),
        banks.source_rate,
        banks.gain,
        20.0 * banks.gain.log10()
    );
    if !banks.provenance_checked {
        eprintln!("WARNING: a manifest is missing; source/passage identity is unverified.");
    } else {
        eprintln!(
            "Manifest source/passage identity and WAV geometry match (PCM hashes not rechecked)."
        );
    }
    if mode == "check" {
        println!(
            "OK: both six-stem banks loaded; all PCM finite and geometry equal. No audio device opened."
        );
        return Ok(());
    }
    let output = Output::open(banks)?;
    if mode == "smoke" {
        return smoke(&output);
    }
    let _terminal = Terminal::enter()?;
    let mut selection = Selection::default();
    loop {
        output.check_device()?;
        draw(&output, selection)?;
        if event::poll(Duration::from_millis(100))?
            && let Event::Key(key) = event::read()?
            && let Some(action) = action_for_key(key)
        {
            if action == Action::Quit {
                break;
            }
            output.action(action)?;
            selection.apply(action);
        }
    }
    Ok(())
}

fn action_for_key(key: KeyEvent) -> Option<Action> {
    if key.kind == KeyEventKind::Release {
        return None;
    }
    // Allow held arrows to seek repeatedly, but avoid double toggles on repeats.
    if key.kind == KeyEventKind::Repeat && !matches!(key.code, KeyCode::Left | KeyCode::Right) {
        return None;
    }
    match key.code {
        KeyCode::Char(c @ '1'..='6') if key.modifiers.is_empty() => {
            Some(Action::ToggleStem(c as usize - '1' as usize))
        }
        KeyCode::Char('0') if key.modifiers.is_empty() => Some(Action::Switch),
        KeyCode::Char(' ') if key.modifiers.is_empty() => Some(Action::PlayPause),
        KeyCode::Left => Some(Action::Seek(-5)),
        KeyCode::Right => Some(Action::Seek(5)),
        KeyCode::Esc => Some(Action::Quit),
        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => Some(Action::Quit),
        _ => None,
    }
}

fn draw(output: &Output, selection: Selection) -> Result<()> {
    let mut stdout = io::stdout().lock();
    execute!(stdout, MoveTo(0, 0), Clear(ClearType::All))?;
    write!(
        stdout,
        "Stem A/B mixer\r\n\r\n{} {}\r\nA: {}\r\nB: {}\r\n\r\n{}  {:.1} / {:.1}s (clip-relative)\r\n\r\n",
        if selection.bank == 0 { "A:" } else { "B:" },
        output.labels[selection.bank],
        output.labels[0],
        output.labels[1],
        if output.playing() {
            "Playing"
        } else {
            "Paused "
        },
        output.position(),
        output.duration
    )?;
    for (index, name) in STEMS.iter().enumerate() {
        write!(
            stdout,
            "{}  {:<7} {}\r\n",
            index + 1,
            name,
            if selection.muted & (1 << index) == 0 {
                "ON"
            } else {
                "MUTED"
            }
        )?;
    }
    write!(
        stdout,
        "\r\n{HELP}\r\nMute selection is shared across A/B.\r\nCommon fixed gain {:.1} dB | {}\r\nDevice underrun frames: {}\r\n",
        20.0 * output.gain.log10(),
        output.device,
        output.shared.underflow_frames.load(Ordering::Relaxed)
    )?;
    stdout.flush()?;
    Ok(())
}

struct Terminal;
impl Terminal {
    fn enter() -> Result<Self> {
        terminal::enable_raw_mode().context("Enabling terminal raw mode")?;
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

fn smoke(output: &Output) -> Result<()> {
    output.action(Action::PlayPause)?;
    for action in [
        Action::Switch,
        Action::ToggleStem(2),
        Action::Seek(5),
        Action::ToggleStem(2),
    ] {
        thread::sleep(Duration::from_secs(1));
        output.action(action)?;
        output.check_device()?;
    }
    output.action(Action::PlayPause)?;
    ensure!(
        output.shared.callbacks.load(Ordering::Relaxed) > 0
            && output.shared.output_frames.load(Ordering::Relaxed) > 0,
        "Device callback consumed no audio"
    );
    println!(
        "Hardware smoke completed: {} callbacks, {} consumed frames, {} underrun frames, {} device errors. This is not a subjective listening check.",
        output.shared.callbacks.load(Ordering::Relaxed),
        output.shared.output_frames.load(Ordering::Relaxed),
        output.shared.underflow_frames.load(Ordering::Relaxed),
        output.shared.device_errors.load(Ordering::Relaxed)
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn keymap_matches_stem_order_and_rejects_release_and_toggle_repeat() {
        for digit in '1'..='6' {
            assert_eq!(
                action_for_key(KeyEvent::new(KeyCode::Char(digit), KeyModifiers::NONE)),
                Some(Action::ToggleStem(digit as usize - '1' as usize))
            );
        }
        assert_eq!(
            action_for_key(KeyEvent::new(KeyCode::Char('0'), KeyModifiers::NONE)),
            Some(Action::Switch)
        );
        assert_eq!(
            action_for_key(KeyEvent::new(KeyCode::Left, KeyModifiers::NONE)),
            Some(Action::Seek(-5))
        );
        assert_eq!(
            action_for_key(KeyEvent::new_with_kind(
                KeyCode::Char('0'),
                KeyModifiers::NONE,
                KeyEventKind::Release
            )),
            None
        );
        assert_eq!(
            action_for_key(KeyEvent::new_with_kind(
                KeyCode::Char('0'),
                KeyModifiers::NONE,
                KeyEventKind::Repeat
            )),
            None
        );
    }
}
