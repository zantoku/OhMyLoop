# Notes screen brief

## Purpose

Keep plain-text notes for one song: what to practise, fingerings, where it
goes wrong, reminders for next time. Writing must be effortless and nothing
may ever be lost.

## Who and how

The same player, mid-practice or just after. Notes are written in short
bursts: stop, write a line about what just went wrong, go back to playing.
Sometimes longer: a practice plan at the start, a summary at the end, lyrics
or chord charts pasted in. Consequences for the design:

- Getting to the notes and back to playing has to be fast; switching never
  disturbs the transport.
- The text gets the space. There's nothing to configure.
- Saving is invisible when it works and impossible to miss when it doesn't.

Frequency tags: **session**: most practice sessions; **often**: several
times per song; **rare**: occasional or recovery.

## User stories

### A. Write notes

| # | Story | Freq. | Actions |
| --- | --- | --- | --- |
| A1 | Open the current song's notes beside the Practice waveform, without stopping or changing playback. | session | `show_notes` |
| A2 | Write and edit plain text; my whitespace and line breaks stay exactly as typed, with spell checking and no formatting. | session | — (editor) |
| A3 | Have my notes saved as I type, and see when they're saved, still saving or unsaved. | session | — |
| A4 | Paste longer text (lyrics, a chord chart, a practice plan) and scroll through it. | rare | — |
| A5 | Close the notes panel; my notes are saved first. | session | `close_notes` |

### B. Practise with notes

| # | Story | Freq. | Actions |
| --- | --- | --- | --- |
| B1 | Read my notes while the song plays, e.g. to follow a plan or lyrics. | rare | — |
| B2 | Pause, play or replay the passage while writing about it, without closing the notes; from the keyboard too, although shortcuts don't fire while typing. | often | Practice transport |

### C. Keep notes safe

| # | Story | Freq. | Actions |
| --- | --- | --- | --- |
| C1 | When saving fails, keep my text, see why, and retry; I can't leave the song, open the Library or quit until it's saved. | rare | `retry_notes` |
| C2 | Know that notes stay with the song when its recording is replaced, and that a separate import of the same recording has its own notes. | rare | — |
| C3 | Be told when notes get too long to save (1 MiB), keeping the last saved version. | rare | — |

## States to design

- **Empty notes** with a placeholder.
- **Writing:** unsaved, saving, saved.
- **Save failed;** notes too long.
- **Long notes** (several screens), scrolled.
- **While playing**, and **while separation runs** (notes stay editable,
  playback is frozen).
- **Panel open and closed,** with the sections sidebar open and closed.
- **No song loaded:** notes are unavailable.
- **Window sizes:** 1180 × 810 default, very large, minimum 1180 × 680 (see [Practice decision 14](practice.md#design-decisions)).
- Light and dark appearance.

## Fixed constraints

- Plain text only: no Markdown, rich text or export in this pass. Up to
  1 MiB of UTF-8.
- One set of notes per song, stored with the song, independent of its
  recording and map. Audio tags are never touched.
- Autosave after 300 ms of quiet. Closing notes, opening the Library,
  changing songs and quitting save pending text first; a failed save blocks
  them until it succeeds.
- Switching between Notes and other views doesn't change the transport.
  Opening the Library pauses playback after notes have saved.
- Keyboard shortcuts never fire while typing.
- Practice and Library decisions carry over: the Stage shell with views as
  a segmented control, toasts for notices, Settings in the Library, rounded
  rectangles throughout, and the
  [Foundations](README.md#foundations-that-carry-over).

## Future

- **Time-pinned notes** ([#17](https://github.com/zantoku/OhMyLoop/issues/17)):
  a line starting with a timecode or measure number shows as a hint on the
  Practice waveform. Out of scope here, but the design shouldn't rule it
  out.

## Open questions for the design

1. **View or panel:** keep Notes as a full view in the segmented control, or
   make it a panel beside the Practice waveform so notes and playback share
   the screen (B1, B2)?
-- Notes is probably simple enough for a side panel. Let's try that first, should also settle questions 2, 3 and 4.
2. **Playback on the Notes view:** if Notes stays a full view, should it
   offer a compact transport (play/pause, position), or is switching to
   Practice enough?
3. **Close button:** with views in a segmented control, is the leading X
   that returns to Practice still needed, or redundant?
4. **Save status:** always visible, or only when unsaved or failed, given
   that the header already shows "✓ Saved"?
5. **Text layout:** a limited line length for readability, or the full
   width? Proportional or monospaced type (chord charts and tab-like text
   line up only in monospace)?
-- Full width since we're in a side panel. Proportional font, but that could get a setting in the shared settings.
6. **Library search:** should searching the Library also match notes?  
-- no

## Design decisions

Answers to the open questions from the story review (2026-11-18):

1. **Side panel:** Notes becomes a panel beside the Practice waveform
   instead of a view in the segmented control, so notes and playback share
   the screen. This also settles questions 2–4.
2. **Playback:** the Practice transport stays in reach while the panel is
   open; there's no separate transport for notes.
3. **No close X:** the panel opens and closes with its toggle; it gets no
   leading X of its own.
4. **Save status:** the header's "✓ Saved" covers the normal case. The panel
   shows status only while saving takes long, or when saving fails.
5. **Text layout:** the text fills the panel's width, in the proportional
   UI font. A monospaced option may come later as a setting in app-wide
   Settings.
6. **Library search** doesn't search notes.

## Chosen concept: tall drawer

Three placements were drawn: a panel right of the waveform (NA), a Notes tab
in the sections sidebar (NB), and a tall drawer (NC). NC was chosen on
2026-11-19.

- **Place:** a 320 px drawer on the right edge of Practice, from below the
  header down to the playback bar, beside both the readout band and the
  waveform. The readout band and waveform narrow while it's open; the
  sections sidebar on the left is unaffected. The height suits long notes
  (lyrics, chord charts, plans).
- **Open and close:** a Notes button in the header, and N, toggle the drawer
  (A1, A5). There's no close X. Notes leaves the Practice / Measure map /
  Stems segmented control.
- **Save status:** the header's "✓ Saved" covers notes and turns into
  "⚠ Notes not saved" when a save fails. The drawer shows "Saving…" only
  when saving is slow, and an alert with Retry when it fails (C1) or when
  the notes exceed 1 MiB (C3).
- **Playing while writing** (B2): shortcuts don't fire while typing, so a
  hint in the drawer says Esc returns focus to the waveform, after which
  Space plays again. The transport stays clickable throughout.
- **Empty notes** show a placeholder in the text area.

Separation running and the minimum window are drawn with the Practice
states; with no song loaded the Library is shown. Still to design: long
notes scrolled.
