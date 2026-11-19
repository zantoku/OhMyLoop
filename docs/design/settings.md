# Settings brief

## Purpose

One place for everything that belongs to the app rather than to a song:
which library is open, the downloadable tools and models, how the app looks,
and how it's controlled. Earlier reviews moved these here from the Library
dialog, the Stems screen and the status bar; this brief collects them.

## Who and how

The same player, rarely. Settings is visited once per computer (choose a
library, install the stem tools), after something broke (a missing library,
damaged tools), or out of curiosity (appearance, shortcuts). Nobody practises
here. Consequences for the design:

- Each section is found by name in one look; nothing is buried two levels
  deep.
- Changes apply at once and are saved; there's no Apply or OK to forget.
- Downloads and removals show their size and cost before they start, and
  their progress while they run.
- Other screens link straight to the right section when they need it
  ("Stem tools are damaged · Open Settings").

Frequency tags: **install**: once per computer; **rare**: occasional or
recovery; **taste**: set once, maybe changed later.

## User stories

### A. Open and leave

| # | Story | Freq. | Actions |
| --- | --- | --- | --- |
| A1 | Open Settings from the Library's Settings button, with ⌘, (Ctrl+, elsewhere) from any screen, or from the macOS app menu. | rare | `show_settings` (new; replaces `managed_settings`) |
| A2 | Land on the section another screen sent me to, e.g. Stem separation when the tools are damaged. | rare | `show_settings` with a section |
| A3 | Leave Settings and be back exactly where I was, with playback state unchanged. | rare | `close_settings` (new), Esc |

### B. Library (moved from the Library review, D1–D3)

| # | Story | Freq. | Actions |
| --- | --- | --- | --- |
| B1 | See which library is open, where it lives, and how many songs it holds. | rare | — |
| B2 | Switch to another library or create a new one; my pending edits are saved first and the current song is closed. | rare | `managed_open`, `managed_create`, `managed_close` |
| B3 | Show the library folder in Finder / Explorer / the file manager. | rare | — (new) |
| B4 | Know how to move or back up a library: close the app, copy the whole folder, then open or locate it. | rare | — |
| B5 | When the library is missing, locate it or open another one here, through the same flow as the Library screen's notice. | rare | `managed_locate`, `managed_open`, `managed_retry` |

### C. Stem separation tools (moved from the Stems review)

| # | Story | Freq. | Actions |
| --- | --- | --- | --- |
| C1 | See whether the separation runtime and model are installed and verified, their versions, and the disk space they use. | rare | `separation_refresh_runtime` |
| C2 | Install them, seeing the download size first and the progress while it runs; cancel or retry. | install | `separation_install` |
| C3 | Repair damaged tools by reinstalling the runtime or the model. | rare | `separation_reinstall_runtime`, `separation_reinstall_model` |
| C4 | Remove the runtime or the model to free disk space, knowing existing stems stay playable. | rare | `separation_remove_runtime`, `separation_remove_model` |

### D. Beat analysis model

| # | Story | Freq. | Actions |
| --- | --- | --- | --- |
| D1 | See whether the beat analysis model is installed, its size, and install it outside the Measure map drafting flow. | rare | `map_install_model` |
| D2 | Remove it to free disk space, knowing existing maps and analysis results stay. | rare | — (new) |

### E. Appearance (moved from the status bar)

| # | Story | Freq. | Actions |
| --- | --- | --- | --- |
| E1 | Choose Auto, Light or Dark; the change shows immediately and survives restart. | taste | — (theme control) |
| E2 | Choose the notes font: proportional or monospaced (so chord charts and tab line up). | taste | — (new) |

### F. Keyboard shortcuts

| # | Story | Freq. | Actions |
| --- | --- | --- | --- |
| F1 | See every shortcut, grouped by screen. | rare | `show_shortcuts` |
| F2 | Find the shortcut file's location, open it, and reload it after editing; see why a file was rejected. | rare | `reload_keymap` |
| F3 | Hide the keyboard-shortcuts button on Practice once I know the shortcuts; the list stays reachable from Settings and its shortcut. | taste | — (new) |

### G. Audio output

| # | Story | Freq. | Actions |
| --- | --- | --- | --- |
| G1 | See which output device is playing. | rare | — |
| G2 | When the device disappears, retry from here as well as from the toast. | rare | `retry_audio` |
| G3 | *Future:* choose the output device instead of the system default ([#18](https://github.com/zantoku/OhMyLoop/issues/18)). | rare | — (new, engine work) |

## States to design

- **Each section** at rest, in light and dark appearance.
- **Library:** open and healthy; missing; switching (pending saves); a
  switch blocked by a failed save or by separation running.
- **Stem tools:** not installed (with sizes), downloading, installed and
  verified, damaged, removing, failed download; blocked while separation
  runs.
- **Beat analysis model:** not installed, downloading, installed, failed;
  blocked while analysis runs.
- **Shortcuts:** default, custom file loaded, file rejected with the reason.
- **Audio:** device playing, no device, device lost.
- **Opened from a link** into a section.
- **Window sizes:** 1180 × 810 default, very large, minimum 1180 × 680 (see
  [Practice decision 14](practice.md#design-decisions)).

## Fixed constraints

- App-wide only: nothing here belongs to one song. Song-level settings
  (measure map numbering, a song's tuning) stay with the song.
- One library open at a time. Switching saves pending edits first; a failed
  save, a running import or a running separation blocks it. A missing
  library is never replaced silently by an empty one. Moving a library
  happens outside the app.
- Model and runtime files are app-wide and reinstallable; removing them never
  removes stems, maps or analysis results. No download starts without an
  explicit action. Separation and analysis share one execution slot.
- The theme choice is stored per computer; an unknown stored value falls
  back to Auto without being overwritten.
- Shortcuts come from `keymap.json`, which the app reads but never writes.
  An invalid file restores all defaults with a warning. See
  [KEYMAP.md](../KEYMAP.md).
- There's no output device chooser today; playback follows the system
  default.
- Every command comes from the shared action model.
- Practice and Library decisions carry over: rounded rectangles, toasts for
  notices, ⌘, and the Library's Settings button as the ways in, and the
  [Foundations](README.md#foundations-that-carry-over).

## Open questions for the design

1. **Form:** a dialog over the current screen (as drawn on the canvas, with
   sections in a sidebar), or a full screen like Library?
-- Can be a dialog, I think
2. **Sections and order:** Library, Appearance, Audio, Shortcuts, Stem
   separation, Beat analysis? Or group the two downloads into one
   "Downloads" or "Tools" section, since they behave alike (size, install,
   repair, remove)?
-- The former, no grouped tools and downloads. Order is fine.
3. **Shortcuts:** move the shortcut list into Settings and drop the header's
   shortcuts button, keep the separate dialog and only link to it, or keep
   both (the header button for a quick look, Settings for the file)?
   Editing shortcuts inside the app stays out of scope either way.
-- Keep both, add an option to hide the button on the practice screen. 
4. **Audio output:** only show the device in use, or add a device chooser
   (new engine work)?
-- Add a design for a device chooser, mark as future improvement, and open a github issue for it
5. **Notes font:** add the monospace option for notes now (Notes decision 5
   said it may come later), or leave it out of this pass?
-- Add a notes font chooser to the appearance section
6. **Blocked actions:** while separation or analysis runs, show blocked
   actions (switch library, remove tools) disabled with the reason, or hide
   them?  
-- We could just disable Settings completely while background actions run

## Design decisions

Answers to the open questions from the story review (2026-11-19):

1. **Form:** a dialog over the current screen.
2. **Sections:** one per topic, in this order: Library, Appearance, Audio,
   Shortcuts, Stem separation, Beat analysis. The two downloads stay
   separate sections.
3. **Shortcuts:** both stay. The header's shortcuts button gives a quick
   look; the Shortcuts section has the file and reload, and an option to
   hide the button on Practice (F3).
4. **Audio output:** this pass shows the device in use and Retry. A device
   chooser is designed now and marked as a future improvement
   ([#18](https://github.com/zantoku/OhMyLoop/issues/18)).
5. **Notes font:** Appearance gets a notes font choice, proportional or
   monospaced (E2). This settles the Notes brief's decision 5.
6. **Blocked actions:** Settings is unavailable as a whole while background
   work runs (stem separation, beat analysis, an import, a download): the
   Settings button and ⌘, are disabled with the reason. A download started
   inside Settings keeps the dialog open with its progress; the other
   sections' actions wait until it ends.

## Chosen concept: sidebar dialog

Three layouts were drawn: a sidebar showing one section at a time (GA), one
scrolling page with jump links (GB), and summary rows that expand in place
(GC). GA was chosen on 2026-11-19.

- **Shape:** a dialog over the current screen with a leading close X (and
  Esc). The six sections sit in a sidebar in the order of decision 2; one
  section shows at a time. Changes apply at once; there's no OK or Apply.
- **Status in the sidebar:** each section name carries a one-line status
  beneath it ("Ready · 1 GB", "Not installed", "Not found", "Custom file"),
  so a problem is visible without opening the section.
- **Opened from a link** (A2), the dialog lands on that section.
- **Library:** the open library with its location and song count, Show in
  Finder, Open another / Create a new library, how to move or back up a
  library, and the missing-library recovery (B1–B5).
- **Appearance:** Auto / Light / Dark, and the notes font (proportional or
  monospaced) with a preview of a chord chart and tab (E1, E2).
- **Audio:** the device in use, and Retry when it's lost (G1, G2). The
  device chooser is drawn in a dashed box marked "Future · #18" (G3).
- **Shortcuts:** "Show the shortcuts button on Practice", View all
  shortcuts, the shortcut file's location with Open and Reload, and its
  status: built-in, custom or rejected with the reason (F1–F3).
- **Stem separation and Beat analysis:** each shows its status, size and
  version; install with the download size, progress with Cancel, repair,
  and remove with the reassurance that stems, maps and analysis results
  stay (C1–C4, D1–D2).
- **During a download** started here, the dialog stays open with progress
  and the other sections' actions are dimmed until it ends (decision 6).

