# Actions and shortcut configuration

Default endpoint navigation is `,` → A, `.` → B, and Enter → song start.
The A and B buttons in the passage bar below the waveform invoke the same
actions (`jump_loop_start`, `jump_loop_end`) when clicked; exact seconds are
entered from the passage card (`show_passage_bounds`).

Seeking to B turns looping off because B is the exclusive end. Seeking to
the song start also turns looping off when it lies outside the enabled range.
Seeking preserves play/pause state; reaching the file's natural end can finish
playback. A range remains saved when looping is turned off.

Practice always pages the waveform to follow the playhead while playing (on
start, at the page's right edge, and on wraps; a manual pan holds until then); its
former `practice_follow` toggle is removed, and a `keymap.json` that still names
it is rejected like any unknown action. Mapping keeps its own `map_follow`.
Explicit seeks reveal the playhead even while paused.

Zoom has no buttons on Practice: the mouse wheel zooms at the pointer. The
keyboard zooms with `zoom_in` (**=**), `zoom_out` (**-**) and `show_whole_song`
(**0**), so zoom works on Windows and Linux, which have no View menu.

The practice actions `practice_metronome` and `toggle_loop_snap` control the
shared authored-map metronome (the On/Off button under the Click knob at the
start of the mixer) and default-on loop snapping (the magnet button beside the
passage bounds). Their buttons are `practice_metronome` and `loop_snap`;
neither has a default key. Both need an accepted measure map. Enable/level
persist across practice/mapping screen changes. Practice uses the accepted map;
mapping can use an unapplied draft. With snapping on, A/B use the nearest
barline, and without an existing loop they select the containing measure. Exact
loop-time inputs and saved sections preserve their source coordinates.

Count-in and looping are independent toggles. The Practice action
`practice_count_in` (**C**, button `count_in`) counts in one measure before
Play. `toggle_loop` (**L**, button `loop`) turns looping on or off and never
changes either count-in. `repeat_count_in` (button `repeat_count_in`, no default
key) counts in before each loop repeat; it is the "each" segment joined to the
Loop button and is available only while looping is on. Both count-ins need an
accepted measure map and are inactive on other screens.

Other Practice actions without default keys: `toggle_sections` (button
`sections`) shows or hides the sections list; `show_pitch` (button `pitch`)
opens the pitch popover with transposition and the recording's reference
tuning; `show_goto_measure` (button `measure_readout`) opens "Go to measure"
from the measure readout; `show_passage_bounds` (button `passage`) opens exact
A/B seconds entry from the passage card. `seek_backward` and `seek_forward` also back the
transport's step buttons (`step_back`, `step_forward`): a measure with a map,
5 seconds without one. `show_practice`, `show_measure_map` and `show_stems` are
the header's view segments and `show_notes` (**N**) its Notes button, which
toggles the notes drawer; they work on every song screen, and from Measure map
or Stems `show_notes` opens Practice with the drawer. `add_section` (**S**,
button `save_selection`) saves the passage as a named section; it moved from N
when N became the notes toggle.

The mapping actions `map_metronome` and `map_follow` control authored-map clicks
and waveform paging. They have button/help metadata and no default keyboard
bindings; custom bindings can be assigned like other mapping actions.
`map_show_detected` toggles the Analysis checkbox for raw beat suggestions and
also accepts a custom keyboard binding. It changes presentation only.

`map_split` splits before the selected numbered beat; `map_merge` joins adjacent
measures with matching denominators. `map_remove` marks a measure as free time,
and `map_reclaim` creates a measure in the selected uncounted range. These actions
use direct validated edits with Undo and share button/hover metadata.
Notation and anchor/boundary edits also commit directly; draft creation and
global numbering retain the Preview/Apply/Discard flow.
`map_shift_resnap` toggles optional detected-beat refitting for subsequent shifts.
Its checkbox uses the same action identity and hover description; it has no default
shortcut and can be toggled even without detected beats. Re-snapping defaults on;
shifting then requires current or restored detection. Epsilon is shared with Create
measures. Configured shift keys honor the checkbox and current tolerance.

`map_shift_left` and `map_shift_right` shift the selected and remaining bars by
the form’s pulse count. They share central button/help metadata and have no
default keys; custom shortcuts work from any selected-measure panel, using the
current form pulse count. Consumed measures are absorbed and Undo restores them. The legacy
`map_respace` identity remains recognized for older keymaps, but is unavailable
in the desktop editor and has no button. Reapply Meter for even beat spacing.

## Configuration file

The app reads `keymap.json` on startup. The exact path is shown under
**Shortcuts → Customize shortcuts**. Usual locations:

- macOS: `~/Library/Application Support/org.ohmyloop.desktop/keymap.json`
- Linux: `${XDG_CONFIG_HOME:-~/.config}/org.ohmyloop.desktop/keymap.json`
- Windows: `%APPDATA%\org.ohmyloop.desktop\keymap.json`

Copy [config/keymap.example.json](../config/keymap.example.json) to that path
and edit it. Use **Reload configuration** in the shortcut dialog to apply
changes without restarting. The application reads configuration without
creating or overwriting it. Missing files use built-in defaults. Invalid JSON,
unsupported versions, unknown fields/actions, conflicting bindings, incompatible
button mappings, or files larger than 64 KiB produce a warning and restore the
complete default keymap. Correct the file and reload to clear the warning.

For isolated development or testing, `OHMYLOOP_KEYMAP_FILE` selects a different
file at application startup. It does not change the song-settings directory.

```json
{
  "version": 1,
  "keys": {
    "play_pause": ["p"],
    "jump_loop_start": ["q"],
    "jump_loop_end": ["w"],
    "jump_song_start": ["Enter"]
  },
  "buttons": {
    "previous_section": "jump_loop_start",
    "next_section": "jump_loop_end"
  },
  "descriptions": {
    "jump_loop_start": "Back to the riff",
    "jump_loop_end": "Continue after the riff"
  }
}
```

`keys` replaces the whole binding list for each named action. Omitted actions
inherit their defaults; `[]` disables a shortcut. Each action may have up to
eight bindings. If a reassigned key was already used, also remove or replace
the other action's binding when their screen scopes overlap. Conflicts include
inherited defaults. Practice-only and mapping-only actions may share a key;
global transport actions overlap both screens.

Bindings use the produced `KeyboardEvent.key` character, independent of physical
keyboard position. Letters are case-insensitive, and Shift may be used to produce
an unmodified character. With Ctrl, Meta, or Alt, Shift is matched explicitly,
so `Mod+p` and `Mod+Shift+p` can invoke different actions. Named keys include
Space, Enter, Escape, ArrowLeft/Right/Up/Down,
Home, End, PageUp/Down, Delete, Backspace, and F1–F12. Use `Plus` for a literal
`+`. Modifiers are `Ctrl`, `Meta`, `Alt`, `Shift`, and `Mod`, separated by `+`;
`Mod` means Command on macOS and Control elsewhere. For example, `Mod+o` opens
a song and `Shift+ArrowLeft` performs fine seeking. Tab remains keyboard focus
navigation. Bindings that overlap after modifier/platform resolution and screen scope are rejected.

`buttons` maps a stable control ID to an action. Actions requiring contextual
data, such as a section, can only replace actions accepting the
same kind of payload. Form submit and dialog-close controls retain their own
payload categories. Layout and icons belong to the view; execution, availability,
tooltips, and remapped accessible labels use the resolved action.

`descriptions` overrides the text used in hover descriptions and shortcut help.
It does not alter action identity or behavior. Descriptions are plain text,
limited to 160 characters. All action/button IDs and default descriptions live
in [ui/actions.ts](../ui/actions.ts); the documentation does not maintain a second
action catalog.

Key ownership is decided by the key (`ui/key-routing.ts`). Text fields,
selects, open dialogs and popovers, and IME composition own every key. Composite controls
(waveform and map seek sliders, loop handles, map lines, the click and stem
knobs, range inputs) own only their documented keys, such as the arrows and Home/End;
letters, Space, Enter and Esc still reach the shortcuts from them. Clicking a
button or toggle leaves keyboard focus where it was, so Space keeps playing;
only a Tab-focused button activates with Space or Enter. Holding a transport
toggle does not repeatedly toggle it; seek actions permit key repeat.

Esc unwinds one level per press: an open popover closes (focus returns to where
it was), then an open dialog closes, a number or text field restores its value
and returns focus to the screen's workspace, any other focused control returns
focus to the workspace, and only then does Esc run its shortcut
(`disable_loop`). Enter in a field commits and returns to the workspace. An
open popover (`data-popover`, such as Pitch or Go to measure) owns the keys
pressed inside it; while focus is elsewhere, shortcuts keep working.

`Mod+z` undoes the last measure-map edit (mapping) or restores a removed section
(practice); `Mod+Shift+z` redoes a measure-map edit. In text fields these keys
keep their native text undo/redo. Outside text fields they are always consumed,
even with nothing to undo, so the webview's own undo never edits an unfocused
field. The macOS Edit ▸ Undo/Redo items follow the same rule (`ui/undo-routing.ts`).
If a custom `keymap.json` binds `Mod+z` (or `Mod+Shift+z`) to another action,
free the default with `"undo_delete": []` and `"map_undo": []` (or
`"map_redo": []`); otherwise the file is rejected as conflicting and all
defaults are restored.

On macOS the menu bar is built from the same actions (`ui/app-menu.ts`): menu
labels, `⌘` accelerators and enabled state follow the resolved keymap, and the
standard About, Services, Hide, Quit and Edit clipboard items are kept. Single-key
shortcuts stay in the webview. Windows and Linux have no menu bar; their Ctrl
shortcuts go through the same webview handler.

## Architecture

- `ui/actions.ts`: stable action IDs, default bindings, descriptions, payload
  categories, requirements, and button-to-action mappings.
- `ui/keymap.ts`: pure configuration validation, override resolution, key
  matching, platform formatting, and descriptions for the view.
- `ui/action-dispatcher.ts`: shared availability, payload checks, and execution.
- `ui/key-routing.ts`: which focused element owns a key; everything else goes to
  the keymap.
- `ui/input-modality.ts`, `ui/home-focus.ts`, `ui/field-commit.ts`,
  `ui/native-chrome.ts`: keyboard/pointer modality for focus rings, the
  workspace's home focus, field commit/revert, and app-wide pointer behaviour
  (no focus on click, no browser context menu, no dragging out).
- `ui/app-menu.ts`: macOS menu bar layout and accelerators from the action model.
- `ui/transient.ts`: open popovers, newest first, so Esc closes them before
  anything else.
- `ui/undo-routing.ts`: Undo/Redo to the focused text field, otherwise the screen's app undo.
- `ui/App.svelte`: state-aware handler adapters and presentation. Its keyboard
  handler resolves a binding and dispatches an action; it contains no key list.
- `src/keymap.rs`: bounded, read-only JSON file loading. It does not know UI
  actions or duplicate their definitions.
- `src-tauri`: resolves the app configuration path and reads the file off the
  UI/audio threads through the command bridge.

The model is replaced atomically after complete validation. Failed reloads do
not apply a partial configuration. A future settings editor can edit the same
versioned format rather than introducing another action or binding system.

## Mapping actions

Mapping actions belong to the `mapping` scope; loop-editing actions belong to
`practice`, and shared transport/navigation actions are available on both.
The shortcut dialog lists actions for the current screen. New mapping commands
have no default bindings so existing keymap files continue to work. For example:

```json
{
  "version": 1,
  "keys": {
    "map_undo": ["Mod+z"],
    "map_redo": ["Mod+Shift+z"],
    "map_analyze": ["r"],
    "map_apply": ["Mod+Enter"]
  }
}
```

Handle nudging remains local widget behavior. Enter/Space on a focused button
activate that button through the native WebView instead of dispatching a global
rewind or playback toggle.

## Library actions

Library actions have stable identities: `show_library`, `library_close`,
`library_open`, `library_edit`, `library_save`, `library_cancel`,
`library_retry`, `library_sort`, `library_reverse`, `library_find` and
`library_clear_search`. Library-only commands are scoped to the Library screen.
Defaults there: **Esc** → `library_close` (back to the current song, as it was
left), **⌘I / Ctrl+I** → `library_edit` (edit the selected song's details) and
**⌘F / Ctrl+F** → `library_find` (focus the search). Esc keeps its
`disable_loop` binding on the song screens: that action's scope is `song`
(every screen but the Library), so the two don't conflict. `library_open` and
`library_replace` need a song-key payload and `library_sort` a field payload,
so those contextual actions can't be bound to keys. `show_library` can be
assigned a global shortcut.

The song list owns ↑/↓, Home/End, Page Up/Down and **Enter** (open the selected
song) while focused; every other key reaches the shortcuts. Esc unwinds one
level per press: a search or details field returns focus to the list, then the
details panel closes (discarding unsaved edits, like Cancel), then the Library
returns to Practice. Every other way out of the Library, and quitting, saves
pending details first; a failed save keeps you on the Library.

`show_settings` (**⌘, / Ctrl+,**, button `settings`) opens Settings from any
screen; it is the macOS app menu's "Settings…". Until app-wide Settings exist
it opens the library dialog, and `managed_settings` (library settings) is an
alias with the same availability. The macOS menu also lists `library_edit` (File →
Edit Song Details…) and `library_find` (Edit → Find Song).

The global Import song command (`open_song`, Cmd/Ctrl+O) first opens the
library, then the file picker. Cancelling keeps the library visible. Dropping
one audio file onto the window runs the same action with the dropped file. The
built-in demo and its `load_demo` action (button `demo`) were removed.

### Older configuration files

A `keymap.json` written for an older version still loads:

- Removed identities (`keys.load_demo`, `descriptions.load_demo`,
  `buttons.demo`, or a button mapped to `load_demo`) are ignored, with a
  warning naming each one.
- Where your own binding uses a key that a newer default now claims (Esc for
  `library_close`, ⌘I/Ctrl+I for `library_edit`, ⌘F/Ctrl+F for `library_find`,
  ⌘,/Ctrl+, for `show_settings`), your binding wins: that default is dropped
  and a warning names it. Bind the action to another key to get it back.

Unknown identities that never existed, and two of your own bindings that
collide (or a binding against an older default), still reject the whole file
and restore the default keymap.
The X beside the mapping panel title binds `show_practice`.

## Notes actions

Notes is a drawer on Practice, not a screen, so there is no `notes` scope.
`show_notes` (**N**) toggles the drawer on every song screen; `close_notes`
closes it and `retry_notes` retries a failed save (the drawer's Retry save
button). Those two have no default keys or buttons of their own besides Retry.
Buttons, hover descriptions and optional keymap overrides use these same stable
identities. Closing flushes edits first; a failed save keeps the drawer open.
The notes text owns every key while focused, so no shortcut fires while typing;
**Esc** in it returns focus to the waveform, and the next key reaches the
shortcuts again. ⌘Z/⇧⌘Z (Ctrl elsewhere) undo and redo in the text. The text
is read-only while the app is busy (opening, importing), so nothing typed then
is lost when the song changes.

N now belongs to `show_notes` in every screen. An existing `keymap.json` that
binds `n` to another action must also unbind it, `"show_notes": []` under
`keys` (or give `show_notes` another key); otherwise the file is rejected as a
conflict and all default shortcuts are used.
