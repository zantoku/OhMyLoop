# Measure map screen brief

## Purpose

Turn a recording into a trustworthy measure map: get a first draft quickly,
listen through it, and fix the places where it's wrong, without touching the
audio or the practice material.

## Who and how

The same player as Practice, but in a different mode. Mapping happens once
per song (occasionally revisited), sitting at the computer with mouse and
keyboard, concentrating. A typical session:

1. **Draft:** analyze the song (or mark a span by hand), choose how to read
   the beats, preview the draft and apply it.
2. **Review:** play through with the metronome clicking the map, watching
   where clicks and barlines drift from the music.
3. **Correct:** stop at a wrong spot, select the measure, fix it, listen
   again. Repeat until the song is done.
4. **Finish:** set numbering or grouping for the whole song if needed, and go
   back to practice.

Steps 2–3 dominate the time spent. Consequences for the design:

- Drafting is a guided, mostly one-off flow. It can be prominent when the
  map is empty and step aside once a map exists.
- During review and correction, the selected measure and the tools for it
  are the focus, close to where the eye already is.
- Precise pointer work on the waveform (dragging lines) is the primary way
  to correct, with keyboard and exact numbers as equal alternatives.

Frequency tags (within a mapping session): **constant**: every few seconds
while correcting; **often**: several times per song; **song**: once per song;
**rare**: occasional, expert or recovery.

## User stories

### A. Get a first draft

| # | Story | Freq. | Actions |
| --- | --- | --- | --- |
| A1 | Have the app detect beats and downbeats, with progress and the option to cancel. | song | `map_analyze`, `map_cancel_analysis` |
| A2 | Install the analysis model once, knowing its size, and still map by hand without it. | rare | `map_install_model` |
| A3 | See detected beats as suggestions on the waveform, clearly different from the accepted map, and hide them when they clutter. | often | `map_show_detected` |
| A4 | Say how to read the detected beats: time signature, which note a beat is, and where the first full measure starts (near the playhead). | song | `map_first_downbeat` |
| A5 | Without analysis, mark a span at the playhead and say how many measures fit, to get an evenly spaced draft. | song | `map_span_start`, `map_span_end`, `map_build` |
| A6 | Preview the draft, see what it found (full bars, partial bars, timing corrections), hear it with the metronome, then apply or discard it. | song | `map_use_analysis`, `map_apply`, `map_discard` |
| A7 | Adjust how closely the draft follows detected beats (tolerance in ms). | rare | — (ε field) |

### B. Review by listening

| # | Story | Freq. | Actions |
| --- | --- | --- | --- |
| B1 | Play and pause, and slow down hard passages. | constant | `play_pause`, speed actions |
| B2 | Hear a metronome that follows the map (or the draft), with accented downbeats, and set its level. | constant | `map_metronome` |
| B3 | See the current measure and beat, and have the view follow playback. | constant | `map_follow` |
| B4 | Step to the previous or next measure, or go to a measure number. | often | `previous_measure`, `next_measure`, `goto_measure`, arrow keys |
| B5 | Zoom to the selected measure or out to the whole song. | often | `map_fit`, `map_whole`, wheel |
| B6 | Turn off a practice loop that gets in the way of listening. | rare | `disable_loop` |

### C. Correct a measure

| # | Story | Freq. | Actions |
| --- | --- | --- | --- |
| C1 | Select a measure by clicking the waveform or its number, and see its number, meter, kind (full, pickup, incomplete) and position. | constant | — (selection) |
| C2 | Drag a barline to where the downbeat really is; nudge it with keys or type an exact position. | constant | `map_boundary` |
| C3 | Fix beat timing inside a measure (rubato, a fermata) by adding, moving or removing beat markers. | often | `map_beat`, `map_remove_beat`, `map_anchor_playhead` |
| C4 | Change a measure's meter, beat note, grouping or kind, optionally for the following measures too. | often | `map_notation` |
| C5 | Split a measure before a beat, or join it with the next one. | often | `map_split`, `map_merge` |
| C6 | When barlines are off by a beat or two from here on, shift this and all following barlines by a number of beats, optionally snapping them to detected beats. | often | `map_shift_left`, `map_shift_right`, `map_shift_resnap` |
| C7 | Mark a measure as free time (uncounted), or turn an uncounted range back into a measure. | rare | `map_remove`, `map_reclaim` |
| C8 | Undo and redo any edit. | constant | `map_undo`, `map_redo` |
| C9 | Lock a measure I've corrected so rebuilds and batch edits leave it alone. | often | `map_protect` |

### D. Whole-song settings

| # | Story | Freq. | Actions |
| --- | --- | --- | --- |
| D1 | Set the number of the first full measure to match my sheet music, with a preview. | rare | `map_origin` |
| D2 | Apply a beat grouping (e.g. 2+2+3) to every measure in one time signature, with a preview. | rare | `map_grouping` |
| D3 | Clear the whole map and start over, with Undo. | rare | `map_clear` |

### E. Move on, and recover

| # | Story | Freq. | Actions |
| --- | --- | --- | --- |
| E1 | Go back to Practice or to another view without losing my draft, selection or undo history. | song | `show_practice`, shell navigation |
| E2 | Understand why an edit was rejected (protected measure, overlap, too many beats) and keep what I typed. | rare | — |
| E3 | Know when analysis is missing, failed, offline or out of date for the current map, and what to do next. | rare | `map_analyze`, `map_install_model` |
| E4 | Understand that analysis waits while stem separation runs (they share one slot). | rare | — |

`map_respace` is a legacy action without UI and stays out of the design.

## States to design

- **Empty map, no model installed:** offer manual drafting and model setup.
- **Empty map, model ready:** analysis not yet run.
- **Analysis running:** progress, Cancel; the rest of the screen stays usable.
- **Suggestions only:** detected beats shown, no accepted map yet.
- **Draft preview:** green draft over the waveform, summary counts, Apply
  and Discard; the metronome follows the draft.
- **Accepted map, nothing selected.**
- **Measure selected:** full, pickup, incomplete, protected.
- **Uncounted (free-time) range selected.**
- **Edit rejected** with an explanation.
- **Stale or missing analysis;** model failure or offline.
- **Separation running:** analysis unavailable; manual edits work.
- **Dense map:** fast songs, 300+ measures, zoomed out.
- **Window sizes:** 1180 × 810 default, very large, minimum 1180 × 680 (see [Practice decision 14](practice.md#design-decisions)).
- Light and dark appearance.

## Fixed constraints

- The map describes the recording. Editing it never stretches audio or moves
  sections or practice loops. A mapping selection is never an A–B loop.
- Detected suggestions, the draft and the accepted map are visibly distinct.
  Suggestions never drive the metronome directly.
- Drafts, numbering and batch grouping use preview → apply. All other edits
  apply directly after validation, each with one Undo. A drag is one edit.
- Undo history (50 edits) survives switching screens in the same session.
- Every pointer edit has keyboard and exact-number equivalents, with visible
  keyboard focus. Validation failures are announced.
- Shift-related settings (pulse count, re-snap, ε) and draft inputs are
  session state and survive screen switches.
- Pitch controls stay on Practice; this screen needs play/pause, position and
  speed.
- Practice decisions carry over where they apply: the Stage shell with views
  as a segmented control, toasts for notices, wheel-only zoom, rounded
  rectangles throughout, and the [Foundations](README.md#foundations-that-carry-over).

## Open questions for the design

1. **Drafting flow:** today Analysis and Create measures live permanently in
   a sidebar. Should drafting be a guided panel that dominates the empty
   state and collapses once a map exists?
-- Yes, I'd like Analysis and Create measures to get out of the way when reviewing and editing a measure map.
2. **Selected-measure tools:** today they sit in a full-width panel below
   the waveform, split into Measure, Join / Split / Shift and Free time.
   Should they be contextual to what's selected (measure, barline, beat,
   uncounted range), e.g. an inspector beside the waveform or a floating
   toolbar near the selection?
-- I'm not sure, we should explore a few options here.
3. **Readout band:** reuse Practice's large measure/beat readout, or a
   compact one to give the waveform more height?
-- More compact, unless we have extra space
4. **Follow playhead:** Practice made following always on. Same here?  
-- yes
5. **Expert controls:** exact frame positions, numeric beat-marker offsets,
   pulse note value, ε. Behind one "Advanced" disclosure, or next to the
   tools they belong to?  
-- I'm not sure - let's explore design variations
6. **Whole-song settings** (numbering, grouping, clear map): a menu, a
   dialog, or a section of the drafting panel?  
-- Settings that can be changed after drafting, like numbering, need to be accessible later. A dialog probably makes sense.
7. **Metronome and detected-beats toggles:** where do they live so their
   state stays visible while reviewing?  
-- detected beats toggle: probably in the bar below the waveform. Metronome can be always on and at 100%.

## Design decisions

Answers to the open questions from the story review (2026-11-17):

1. **Drafting:** Analysis and Create measures get out of the way once a map
   exists. They lead the empty state and are reopened only to redraft.
2. **Selected-measure tools:** a floating toolbar anchored to the selected
   measure (concept MB, chosen 2026-11-18). See
   [Chosen concept](#chosen-concept-floating-toolbar).
3. **Readout band:** compact, giving the waveform the height, unless a
   layout has room to spare.
4. **Follow playhead:** always on; the `map_follow` toggle is removed.
5. **Expert controls:** in the toolbar's ⋯ panel, next to the tools they
   belong to rather than behind a global "Advanced" disclosure.
6. **Whole-song settings:** numbering, grouping and clear map stay
   reachable after drafting, in a dialog.
7. **Toggles:** the detected-beats toggle sits in the bar below the
   waveform. The metronome is always on at full level on this screen, so
   the `map_metronome` toggle and its level control are removed.

## Chosen concept: floating toolbar

Three placements were drawn: an inspector beside the waveform (MA), a
floating toolbar at the selection (MB) and a contextual strip under the
waveform (MC). MB was chosen on 2026-11-18 for its minimalism: the tools sit
where the eye already is, and the waveform keeps its full width and height.

- **Anchoring:** the toolbar floats over the waveform, just below the ruler,
  centred on the selected measure. It doesn't cover the ruler band, so
  measure numbers and the playhead marker stay visible. It clamps to the
  edges of the view, fades while a line is dragged, and is reachable by
  keyboard focus. When the selected measure scrolls out of view, it
  collapses to an edge tab ("m. 37 ›") that scrolls back to the measure.
- **Contents, in order:** measure number; measure properties ("4/4 · Full
  ▾"); Split; Join; Shift ◀ ▶; Lock; ⋯.
- **Split** (C5) splits before the beat nearest the playhead, so it needs
  no extra input. The button names that beat ("Split at beat 3") and a
  preview line marks it on hover. It's disabled when the playhead is outside
  the selected measure. After a split, a toast confirms the result and that
  later measures were renumbered, with Undo.
- **Measure properties panel** (C4): meter, kind (full, pickup,
  incomplete), how long a pickup lasts, grouping, the beat note, and "apply
  to following measures", with Cancel and Apply.
- **⋯ panel** (expert controls, C2, C3, C6, C7): exact barline positions,
  beat-marker offsets, the shift step with re-snap and ε, and Mark as free
  time.
- **Beats** show as dotted lines across the full waveform height, so they
  can be aligned with waveform peaks. On the selected measure, diamonds on
  the beat lines are the drag handles.

**Drafting drawer** (2026-11-19): on an empty map, the drafting steps sit in
a full-height drawer on the right, from below the header down to the
playback bar, like Notes on Practice.

Still to design: free-time range selected, edit rejected, stale or missing
analysis, separation running, dense map.
