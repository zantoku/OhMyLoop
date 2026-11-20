# Stems screen brief

## Purpose

Give a song its stems: split the recording into instrument tracks once, or
attach a set made elsewhere, so Practice can mute, solo or turn down single
instruments. Show what the song has, and keep the separation tools working.

## Who and how

The same player, but this is a visit, not a workplace. A song gets stems once
(occasionally again). Separation takes minutes, so the player starts it and
then waits, often looking at something else: notes, the measure map, or the
instrument. Consequences for the design:

- The screen leads with one obvious next step: set up, separate, wait, or go
  back to Practice.
- Progress must be readable at a glance, also from the other screens, and
  must be honest: no fabricated percentages or ETAs.
- One-off setup, repair and manual attachment are secondary. They must stay
  findable, but not compete with the main step.
- Mixing itself happens in Practice ([practice.md](practice.md), C6), not
  here.

Frequency tags: **song**: once per song; **install**: once per computer;
**rare**: occasional, expert or recovery.

## User stories

### A. Get stems

| # | Story | Freq. | Actions |
| --- | --- | --- | --- |
| A1 | Set up separation once, knowing the download size and that it then works offline; follow progress, cancel, retry. | install | `separation_install`, `separation_cancel` |
| A2 | Separate the current song with one action, knowing beforehand the disk space needed, that it uses several GB of memory, and that playback pauses until it finishes. | song | `separation_start` |
| A3 | Follow the work: one overall bar, the current step, the step checklist and elapsed time. Steps without measurable progress show as working, not as a made-up percentage. | song | — |
| A4 | Cancel before saving begins, keeping any stems the song already had. | rare | `separation_cancel` |
| A5 | When it's done, see which tracks were kept and which near-silent ones were skipped, and go back to Practice. | song | — (`separation_practice` dropped with Go to Practice, decision 8) |
| A6 | Leave the screen while it runs, edit notes or the measure map, see that work continues from anywhere, and come back to it. | song | view switcher (`show_practice`), `separation_open` |
| A7 | Separate again to replace the current set, after confirming; the old set stays until the new one is saved, and the mix for matching instruments is kept. | rare | `separation_start` |
| A8 | Attach a stem set made elsewhere (an output folder with its manifest), with the same progress checklist. | rare | `attach_stems` |

### B. Know what this song has

| # | Story | Freq. | Actions |
| --- | --- | --- | --- |
| B1 | See whether the song has stems, which instruments, and whether they were separated here or attached. | song | — |
| B2 | Know that playback is ready, or that it's falling back to the original recording because stems are missing or invalid (my mix is kept). | rare | — |
| B3 | See the shared headroom applied to keep the mix from clipping. | rare | — |
| B4 | Remove the stems, after confirming; the recording, measure map, sections and notes stay. | rare | `remove_stems` |

### C. Look after the separation tools

| # | Story | Freq. | Actions |
| --- | --- | --- | --- |
| C1 | See whether the separation tools and the model are installed and working. | rare | `separation_refresh_runtime` |
| C2 | Repair a broken installation by reinstalling the tools or the model. | rare | `separation_reinstall_runtime`, `separation_reinstall_model` |
| C3 | Free disk space by removing the tools or the model, knowing that existing stems stay. | rare | `separation_remove_runtime`, `separation_remove_model` |

### D. Recover

| # | Story | Freq. | Actions |
| --- | --- | --- | --- |
| D1 | When setup or separation fails, read a short explanation, retry, and copy the details for a bug report. | rare | `separation_install`, `separation_start` |
| D2 | Learn before starting that there isn't enough disk space. | rare | — |
| D3 | Understand that separation waits while beat analysis runs (they share one slot), and that analysis waits for separation. | rare | — |
| D4 | Understand when separation isn't supported on this computer, and that attaching stems made elsewhere still works. | rare | `attach_stems` |
| D5 | When saving the stem mix fails, retry without losing it. | rare | `retry_stem_mix` |

### E. Move on

| # | Story | Freq. | Actions |
| --- | --- | --- | --- |
| E1 | Go back to Practice (or another view) at any time without cancelling work in progress. | song | view switcher (`show_practice`, `show_measure_map`) |

## States to design

- **Separation unsupported** on this computer.
- **Setup required;** setup running (download, verify, install); setup
  failed or cancelled.
- **Ready, song has no stems.**
- **Waiting:** analysis is running; separation can't start yet.
- **Separating:** measured step, unmeasured step, Stopping, Finishing save
  (Cancel no longer possible).
- **Done:** kept and skipped tracks, Go to Practice.
- **Failed or cancelled,** with and without an existing set preserved.
- **Stems attached,** separated or imported; playback ready or falling back
  to the original.
- **Attaching manually,** with its checklist.
- **Confirmations:** replace stems, remove stems, remove tools, remove model.
- **Not enough disk space.**
- **Seen from elsewhere:** Practice and Measure map while separation runs
  (playback frozen, indicator on the Stems view).
- **Window sizes:** 1180 × 810 default, very large, minimum 1180 × 680 (see [Practice decision 14](practice.md#design-decisions)).
- Light and dark appearance.

## Fixed constraints

- The reference recording stays the source for the waveform and the measure
  map. Separation and attachment never change the recording, map, sections
  or notes.
- One song has at most one stem set. Replacing takes effect only after the
  new set is validated, copied and saved; failure or cancellation keeps the
  old one. Matching instruments keep their gain and mute.
- Opening the screen starts no work. Setup stops at Ready; separation is a
  separate action.
- One model (Roformer-SW) and one CPU profile: no model, device or quality
  chooser.
- Progress is honest: weighted stage progress with measured steps, no ETA,
  unmeasured steps indeterminate. The overall bar has the fixed title
  "Splitting song into stems", with the current phase as its subtitle.
  Checklist labels are short (≤ 8 words, no file names), the list follows
  the active step unless the user scrolls away, and circular progress starts
  at 12 o'clock and fills clockwise.
- While separation runs, playback, seeking, speed, pitch, loop and the mixer
  are frozen, and song or library can't change; waveform browsing, notes and
  manual map editing still work. Playback stays paused afterwards.
- Analysis and separation share one slot, without a queue.
- Quitting during separation asks to cancel, or finishes a save already
  under way.
- Diagnostics belong to the current attempt only and aren't stored.
- Practice decisions carry over: the Stage shell with views as a segmented
  control, toasts for notices, rounded rectangles throughout, and the
  [Foundations](README.md#foundations-that-carry-over).

## Open questions for the design

1. **Screen or not:** Stems is visited about once per song. Should it stay a
   full view in the segmented control, become a dialog or side panel over
   Practice, or start from the Practice mixer (e.g. an empty mixer offering
   "Separate song")?
-- Yes. A separate screen will give us more space to contain the complexity here.
2. **Setup and separation:** keep two explicit steps (set up, then
   separate), or let "Separate song" run setup first when needed, with one
   combined progress?
-- Setup takes long enough to warrant a separate step.
3. **Progress elsewhere:** while separation runs and I'm on another view,
   how do I see it: a ring on the Stems segment, a persistent toast, a strip
   in the status bar, or several of these?
-- Permanent toast sounds right here.
4. **Checklist:** always show the full step list, or show the bar and
   current step with the list behind "Show steps"?
-- Hiding the details is probably correct if the user can't act on them anyway. They should become available in the error case.
5. **Tool management** (reinstall, remove, model location): on this screen
   behind a disclosure, or in an app-wide settings place, since the tools
   are shared by all songs?
-- App-wide settings seems correct here
6. **Manual attachment:** keep it as a secondary action on this screen, or
   move it to a menu?
-- secondary action here
7. **Set details:** which details earn space (instruments, source,
   playback state, headroom), and which go behind a disclosure?
-- your judgement call, but generally hide unneccesary complexity
8. **Done:** stay on the result with Go to Practice (today), or return to
   Practice automatically with a toast summarizing kept and skipped tracks?
-- stay, no automatic navigation

## Design decisions

Answers to the open questions from the story review (2026-11-18):

1. **Screen:** Stems stays a full view in the segmented control. The space
   holds the setup, progress, result and recovery states.
2. **Setup and separation:** two explicit steps. Setup takes long enough to
   be its own action; it stops at Ready.
3. **Progress elsewhere:** a persistent toast on the other views while
   setup or separation runs, with its progress and a way to open Stems.
4. **Checklist:** the overall bar, its phase subtitle and elapsed time stay
   visible. The step list sits behind "Show steps" while work runs, for
   anyone who wants it, and is open when work fails or is cancelled, with
   the details to copy.
5. **Tool management:** reinstalling, removing and checking the tools and
   the model move to an app-wide settings place, since they're shared by
   all songs. The Stems screen only links there when setup is broken.
6. **Manual attachment:** stays on this screen as a secondary action.
7. **Set details:** show the instruments (with skipped ones marked) and
   where the set came from. Playback state appears only when it falls back
   to the original. Headroom and the model name go behind a details
   disclosure.
8. **Done:** no separate result state and no automatic navigation. After
   a separation, the screen shows the attached stems with a dismissible
   "Separation finished" banner. "Go to Practice" is dropped as redundant.

## Chosen concept: guided steps

Three layouts were drawn: a status card (SA), instrument lanes (SB) and
guided steps (SC). SC was chosen on 2026-11-18.

- A step list on the left (Set up separation → Separate this song → Review
  the stems) shows where the song stands; the pane beside it holds the
  current step. Done steps show a check and a one-line summary.
- Attaching stems from another tool, and replacing or removing a set, are
  quiet links under the steps.
- While work runs, the pane shows the progress bar, phase, elapsed time,
  Cancel and "Show steps". The persistent toast takes over on other views.

## Future: stem review

Tracked in [#15](https://github.com/zantoku/OhMyLoop/issues/15). Once a
song has stems, the pane lists one row per stem with a shared playhead:
play it alone, its name (renameable), its waveform, show or hide it in the
Practice mixer, and a ⋯ menu with Rename, Hide in mixer and Delete.
Near-silent stems are kept and flagged "probably empty" with a direct
Delete: the player decides, replacing today's automatic skip below
−60 dBFS.
