// SPDX-License-Identifier: GPL-3.0-or-later
use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Action {
    Mute(usize),
    Solo,
    Gain(f32),
    Role(i32),
    Source,
    PlayPause,
    Seek(f64),
    LoopStart,
    LoopEnd,
    LoopClear,
    Speed(f64),
    Pitch(f64),
    Metronome,
    CountIn,
    Quit,
}
struct Binding {
    key: KeyCode,
    action: Action,
    help: &'static str,
    repeat: bool,
}
const BINDINGS: &[Binding] = &[
    Binding {
        key: KeyCode::Char('0'),
        action: Action::Source,
        help: "0 reference/A/B",
        repeat: false,
    },
    Binding {
        key: KeyCode::Char(' '),
        action: Action::PlayPause,
        help: "Space play/pause",
        repeat: false,
    },
    Binding {
        key: KeyCode::Left,
        action: Action::Seek(-5.0),
        help: "Left/Right seek 5s",
        repeat: true,
    },
    Binding {
        key: KeyCode::Right,
        action: Action::Seek(5.0),
        help: "",
        repeat: true,
    },
    Binding {
        key: KeyCode::Tab,
        action: Action::Role(1),
        help: "Tab select role",
        repeat: false,
    },
    Binding {
        key: KeyCode::Char('s'),
        action: Action::Solo,
        help: "s solo selected role",
        repeat: false,
    },
    Binding {
        key: KeyCode::Char('-'),
        action: Action::Gain(-0.1),
        help: "-/= gain -/+10%",
        repeat: true,
    },
    Binding {
        key: KeyCode::Char('='),
        action: Action::Gain(0.1),
        help: "",
        repeat: true,
    },
    Binding {
        key: KeyCode::Char('['),
        action: Action::LoopStart,
        help: "[ start / ] end+enable loop / l clear",
        repeat: false,
    },
    Binding {
        key: KeyCode::Char(']'),
        action: Action::LoopEnd,
        help: "",
        repeat: false,
    },
    Binding {
        key: KeyCode::Char('l'),
        action: Action::LoopClear,
        help: "",
        repeat: false,
    },
    Binding {
        key: KeyCode::Char(','),
        action: Action::Speed(-0.25),
        help: ",/. speed -/+0.25",
        repeat: true,
    },
    Binding {
        key: KeyCode::Char('.'),
        action: Action::Speed(0.25),
        help: "",
        repeat: true,
    },
    Binding {
        key: KeyCode::Char(';'),
        action: Action::Pitch(-1.0),
        help: ";/' pitch -/+1 semitone",
        repeat: true,
    },
    Binding {
        key: KeyCode::Char('\''),
        action: Action::Pitch(1.0),
        help: "",
        repeat: true,
    },
    Binding {
        key: KeyCode::Char('m'),
        action: Action::Metronome,
        help: "m metronome / c count-in (requires --bpm)",
        repeat: false,
    },
    Binding {
        key: KeyCode::Char('c'),
        action: Action::CountIn,
        help: "",
        repeat: false,
    },
    Binding {
        key: KeyCode::Esc,
        action: Action::Quit,
        help: "Esc/Ctrl-C quit",
        repeat: false,
    },
];
pub fn help() -> String {
    std::iter::once("1-6 mute bass/drums/guitar/piano/vocals/other")
        .chain(BINDINGS.iter().map(|b| b.help).filter(|h| !h.is_empty()))
        .collect::<Vec<_>>()
        .join(" | ")
}
pub fn action(key: KeyEvent) -> Option<Action> {
    if key.kind == KeyEventKind::Release {
        return None;
    }
    if key.code == KeyCode::Char('c') && key.modifiers == KeyModifiers::CONTROL {
        return Some(Action::Quit);
    }
    if !key.modifiers.is_empty() {
        return None;
    }
    if let KeyCode::Char(c @ '1'..='6') = key.code {
        return (key.kind != KeyEventKind::Repeat)
            .then_some(Action::Mute(c as usize - '1' as usize));
    }
    BINDINGS
        .iter()
        .find(|b| b.key == key.code && (key.kind != KeyEventKind::Repeat || b.repeat))
        .map(|b| b.action)
}
