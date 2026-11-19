# Practice screen brief

## Purpose

Hear a song at a workable speed and pitch, find the passage that needs work,
loop it until it's right, and get back to it next time.

## Who and how

A guitarist (or other instrumentalist) with the instrument in their hands. The
computer is often an arm's length away or further, so hands go to the keyboard
briefly or not at all. During a loop they mainly *look*: where am I, which
measure, how much is left. They *operate* the screen between attempts.
Consequences for the design:

- Position, measure and loop state must be readable at a distance.
- Every frequent task has a single-key shortcut and needs no pointer precision.
- Controls used between attempts (speed, loop bounds) are visible without
  opening anything.

Frequency tags: **constant**: every few seconds while practising;
**session**: several times per practice session; **song**: once when
setting up a song; **rare**: occasional or recovery.

## User stories

### A. Play and keep my place

| # | Story | Freq. | Actions |
| --- | --- | --- | --- |
| A1 | Start and stop playback without looking. | constant | `play_pause` (Space) |
| A2 | See where I am in the song: time, and measure and beat when the song has a measure map. | constant | — (transport status) |
| A3 | See the playhead on the waveform; the view always follows it while playing. | constant | — (`practice_follow` removed) |
| A4 | Click anywhere on the waveform to play from there. | session | — (waveform seek) |
| A5 | Step back or forward by one measure, or 5 s when unmapped; nudge by 0.1 s. | constant | `seek_backward`, `seek_forward`, `seek_backward_fine`, `seek_forward_fine`, `previous_measure`, `next_measure` |
| A6 | Jump back to the first measure, or to the song start. | session | `jump_song_start` (Enter) |
| A7 | Go to a measure number I read from the sheet music. | rare | `goto_measure` |

### B. Loop a passage

| # | Story | Freq. | Actions |
| --- | --- | --- | --- |
| B1 | Drag across the waveform to choose a passage; it snaps to whole measures unless I turn snapping off or hold Shift. | session | `toggle_loop_snap` |
| B2 | While listening, mark the passage start and end at the playhead. | session | `set_loop_start` (A), `set_loop_end` (B) |
| B3 | Fine-tune either end by dragging, with arrow keys (next barline, or fine with Shift), or by typing exact seconds. | session | — (loop handles, inputs) |
| B4 | Turn looping on and off, and separately choose a one-bar count-in before each repeat; see both at a glance. | constant | `toggle_loop` (L), `repeat_count_in`, `disable_loop` (Esc) |
| B5 | Jump to the passage start or end. | session | `jump_loop_start` (,), `jump_loop_end` (.) |
| B6 | See how long the passage is. | session | — |
| B7 | Zoom the view to fit the passage. | session | `fit_loop` |
| B8 | Clear the passage. | session | `clear_loop` (X) |

### C. Shape what I hear

| # | Story | Freq. | Actions |
| --- | --- | --- | --- |
| C1 | Slow down or speed up in 5/10 % steps or to an exact percentage, and reset to 100 %. | session | `speed_down_10`, `speed_down_5`, `speed_up_5`, `speed_up_10`, `reset_speed` |
| C2 | Transpose by semitones and reset. | song | `transpose_down`, `transpose_up`, `reset_pitch` |
| C3 | Correct a recording tuned away from A = 440 Hz by entering its reference pitch (e.g. A = 432 Hz). | song | `reset_tuning` |
| C4 | Hear a metronome click following the measure map, and set its level. | session | `practice_metronome` |
| C5 | Get a one-measure count-in before playback starts. | session | `practice_count_in` (C) |
| C6 | With stems attached, mute, solo or turn down individual instruments, e.g. mute guitar to play along. | session | `stem_gain`, `mute_*` (1–6), `solo_*` |

Speed, pitch, tuning and the stem mix are saved per song. Metronome and
follow settings last for the app session.

### D. Keep named sections

| # | Story | Freq. | Actions |
| --- | --- | --- | --- |
| D1 | Save the current passage as a named section (e.g. "Solo, bars 33–40"). | song | `add_section` (N), `save_section`, `close_editor` |
| D2 | See all sections in song order with their times, and where they lie on the timeline. Sections may overlap or nest. | session | — |
| D3 | Jump to a section; with looping on, it becomes the loop. | session | `select_section` |
| D4 | Step to the previous or next section. | session | `previous_section` ([), `next_section` (]) |
| D5 | Rename a section or adjust its bounds. | rare | `edit_section` |
| D6 | Delete a section, with Undo. | rare | `delete_section`, `undo_delete` |

### E. Move around the song view

| # | Story | Freq. | Actions |
| --- | --- | --- | --- |
| E1 | Zoom in and out around the pointer or with buttons, and show the whole song. | session | `zoom_in`, `zoom_out`, `show_whole_song` |
| E2 | See the whole song at a glance and move the detailed view by clicking or dragging. | session | — (minimap) |
| E3 | Pan without moving the playhead or loop (middle-drag). | session | — |
| E4 | Read measure numbers (or timecode in unmapped audio) above the waveform at any zoom. | constant | — (ruler) |

### F. Go elsewhere

| # | Story | Freq. | Actions |
| --- | --- | --- | --- |
| F1 | Open the library to pick another song (pauses playback). | song | `show_library` |
| F2 | Import a song file directly. | rare | `open_song` (Mod+O) |
| F3 | Open this song's notes, measure map or stems. | song | `show_notes`, `show_measure_map`, `show_stems` |
| F4 | Look up keyboard shortcuts and reload a customized shortcut file. | rare | `show_shortcuts`, `close_shortcuts`, `reload_keymap` |
| F5 | Switch appearance between auto, dark and light. | rare | — (theme control) |

### G. Know the app is OK, and recover when it isn't

| # | Story | Freq. | Actions |
| --- | --- | --- | --- |
| G1 | Trust that my settings are saved, and see which audio device is playing. | rare | — |
| G2 | When the audio device disappears, see why playback stopped and retry. | rare | `retry_audio` |
| G3 | When saving practice settings or the stem mix fails, retry without losing them. | rare | `retry_save`, `retry_stem_mix` |
| G4 | See that stem separation or analysis is running elsewhere and open it. While separation runs, playback controls are frozen and I should understand why. | rare | `separation_open` |
| G5 | Learn that my shortcut file is invalid and defaults are active; reload after fixing it. | rare | `reload_keymap` |
| G6 | Dismiss a message I've read. | rare | `dismiss_error` |

### H. Start without a song

| # | Story | Freq. | Actions |
| --- | --- | --- | --- |
| H1 | On first launch, understand what the app does and get to a song: library or import. (The built-in demo is removed; see [library.md](library.md#design-decisions).) | rare | `show_library`, `open_song` |

## States to design

- **No song:** the Library is shown instead (decision 12).
- **Song without a measure map:** ruler shows timecode; no snapping, count-in,
  metronome or measure navigation. Explain how to enable them (Measure map)
  rather than hiding them silently.
- **Song with a measure map,** including free-time gaps and a pickup measure.
- **No passage / passage set with loop off / loop on / loop with count-in.**
- **Playing, paused, counting in.**
- **No sections; many sections (20+); overlapping sections.**
- **With a stem set (up to six tracks) and without.**
- **Separation running:** playback, speed/pitch, loop and mixer frozen;
  waveform browsing still works.
- **Errors:** audio device lost, save failed, invalid shortcut file; several
  at once.
- **Window sizes:** 1180 × 810 default, very large (2200 × 1300), minimum
  1180 × 680 (decision 14).
- **Very long song** (10+ min) and very short passage (< 1 s).
- Light and dark appearance.

## Fixed constraints

- Loop bounds and sections are positions on the reference timeline,
  independent of speed and pitch. A passage is temporary; a section is saved.
  They are separate concepts, even when a section is selected as the loop.
- Snapping affects pointer gestures only; typed bounds stay exact.
- Seeking to B turns looping off; X clears the passage and turns looping off.
- Opening the library pauses playback; Notes, Measure map and Stems keep it.
- All commands remain bindable through the keymap, with descriptions in
  tooltips and the shortcut list.
- The [Foundations](README.md#foundations-that-carry-over) apply.

## Design decisions

Answers to the open questions from the story review:

1. **Sections:** the timeline lane stays; the sidebar list may collapse.
2. **Passage bounds:** with a measure map, show A/B as measures and beats.
   Seconds remain for unmapped audio and exact entry.
3. **Toggles:** snap to measures sits near the waveform. Metronome and
   count-in sit near the play controls. Follow playhead is always on, and
   its toggle (`practice_follow`) is removed.
4. **Navigation:** open. Explore a few hierarchy concepts for how Library,
   Notes, Measure map and Stems relate to Practice.
5. **Notices:** use toasts. Errors needing action persist until resolved or
   dismissed; informational notices time out.
6. **Mixer:** keep the rotary gain knobs; the placement and layout of mixer,
   transport and mode buttons are open.

Design review (2026-11-17) chose the **Stage** concept: a large measure,
loop and speed readout above the waveform, with views as a segmented control.

7. **Sections:** a collapsible sidebar beside the waveform, plus the lane.
8. **Metronome:** the click is a rotary knob with an on/off button at the
   start of the mixer, not a separate button and slider.
9. **Zoom:** mouse wheel only; no zoom buttons on screen. The zoom actions
   stay available for keyboard bindings.
10. **Pitch:** a tile beside Speed in the readout band shows transposition
    and the recording's reference tuning (e.g. "−2 st", "A 432 Hz"). Each
    line is highlighted only when changed from its default, otherwise muted.
    Clicking opens a popover with the transpose stepper and a "Recording
    tuned to A = … Hz" field; the engine still applies cents
    (±50 ¢ ≈ A 427.6–452.9 Hz).
11. **Loop and count-in:** two independent concerns. Count-in (C) means
    only "before Play". Loop (L) is a plain on/off toggle. A segment
    joined to the Loop tile switches a one-bar count-in before each repeat
    (`repeat_count_in`); it is disabled while looping is off. Neither
    control changes the other, which replaces today's three-state loop
    cycle. The loop card in the readout band notes "count-in each repeat".

States review (2026-11-19), drawn on the canvas's "Chosen designs" page:

12. **No song loaded:** the song views (Practice, Measure map, Stems) don't
    exist without a song. When no song is loaded (first launch, after
    deleting the current song, after switching libraries) the app shows the
    Library.
13. **Song without a measure map:** the readout shows the time ("1:24.6 of
    3:48") with an "Add a measure map" link; loop, sections and A/B points
    show times, and the waveform grid shows seconds. Snap to measures is
    hidden; the step buttons jump 5 s. Count-in, count-in each repeat and
    the click stay visible but disabled, each saying it needs a measure map.
14. **Minimum window 1180 × 680:** every element stays visible, including
    six stems plus the click in the mixer and both side panels (sections and
    notes) open; the waveform gives up height first. Narrow layouts are out
    of scope for now. This raises today's 820 × 620 minimum in
    `src-tauri/tauri.conf.json` when implemented.
15. **No stems:** the mixer shows the click and one Song volume, with a
    "Split into stems" link to the Stems view.
16. **Separation running:** play, the step buttons and the mixer are
    disabled; a progress toast says playback is paused until it finishes.
    Notes stay editable.
