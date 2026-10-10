# Actions and shortcut configuration

Default endpoint navigation is `,` → A, `.` → B, and Enter → song start.
The A/B labels beside the time fields also invoke the same actions when clicked.

Seeking to B turns looping off because B is the exclusive end. Seeking to
the song start also turns looping off when it lies outside the enabled range.
Seeking preserves play/pause state; reaching the file's natural end can finish
playback. A range remains saved when looping is turned off.

The practice action `practice_follow` and its same-named button toggle paged
playhead following. It defaults on, has no default key, and accepts configured
shortcuts. Its preference is independent of mapping’s `map_follow`; each screen
retains its setting across screen changes. Panning and zoom never turn it off,
and explicit seeks reveal the playhead even with continuous following off.

The practice actions `practice_metronome` and `toggle_loop_snap` control the
shared authored-map metronome and default-on loop snapping. Their buttons are
`practice_metronome` and `loop_snap`; neither has a default key. Enable/level
persist across practice/mapping screen changes. Practice uses the accepted map;
mapping can use an unapplied draft. With snapping on, A/B use the nearest
barline, and without an existing loop they select the containing measure. Exact
loop-time inputs and saved sections preserve their source coordinates.

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
data, such as a section or speed preset, can only replace actions accepting the
same kind of payload. Form submit and dialog-close controls retain their own
payload categories. Layout and icons belong to the view; execution, availability,
tooltips, and remapped accessible labels use the resolved action.

`descriptions` overrides the text used in hover descriptions and shortcut help.
It does not alter action identity or behavior. Descriptions are plain text,
limited to 160 characters. All action/button IDs and default descriptions live
in [ui/actions.ts](../ui/actions.ts); the documentation does not maintain a second
action catalog.

Practice shortcuts are suspended inside editable fields and dialogs and during
IME composition. Holding a transport toggle does not repeatedly toggle it;
seek actions permit key repeat. Standard widget keyboard behavior (waveform
slider navigation, loop-handle adjustment, and numeric input editing) remains
local to those widgets.

## Architecture

- `ui/actions.ts`: stable action IDs, default bindings, descriptions, payload
  categories, requirements, and button-to-action mappings.
- `ui/keymap.ts`: pure configuration validation, override resolution, key
  matching, platform formatting, and descriptions for the view.
- `ui/action-dispatcher.ts`: shared availability, payload checks, and execution.
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
