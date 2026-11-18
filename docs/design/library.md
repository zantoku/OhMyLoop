# Library screen brief

## Purpose

Keep the songs I practise in one place: find one and open it, add new
recordings, and keep their names right. The library folder itself is chosen
and switched in app-wide Settings; this screen only reports when it breaks.

## Who and how

The same player, between practice sessions. Most visits are short: open the
app, pick the song for today (usually one played recently), and go to
Practice. Adding a song happens when a new piece comes up; correcting its
details happens right after import, if at all. Choosing, moving or repairing
the library folder happens once per computer or after something went wrong.
Consequences for the design:

- Finding and opening a song is the main task and should take one or two
  actions, with recently opened songs easy to reach.
- Import is the second task: always visible, never in a menu.
- Editing details, replacing a recording and deleting are per-song and
  occasional. They stay out of the list until a song is chosen.
- Choosing, switching and moving the library are rare and shared by the
  whole app, so they live in app-wide Settings. When the library breaks,
  this screen shows a notice with the way out.

Frequency tags: **visit**: most times the Library is opened; **song**: once
per song (adding or describing it); **install**: once per computer; **rare**:
occasional, expert or recovery.

## User stories

### A. Find and open a song

| # | Story | Freq. | Actions |
| --- | --- | --- | --- |
| A1 | See my songs with title, artist and length, the recently opened ones first, and recognise the song that's currently loaded. | visit | — |
| A2 | Find a song by typing part of its title, artist, album or file location. | visit | — (search field) |
| A3 | Sort by title, artist, length, date added or last opened, and reverse the order. | rare | `library_sort`, `library_reverse` |
| A4 | Open a song for practice; it starts paused at its saved position, with its sections, measure map and notes. | visit | `library_open` |
| A5 | Go back to the current song without choosing another. | visit | `library_close` |

### B. Add songs

| # | Story | Freq. | Actions |
| --- | --- | --- | --- |
| B1 | Import a recording, learning once that it's copied into the library and the original stays untouched; follow the copy, then land in Practice. | song | `open_song`, `managed_continue` |
| B2 | On the first import, create a library folder or open an existing one, without having to understand the folder layout. | install | `managed_create`, `managed_open`, `managed_close` |
| B3 | When the exact recording is already in the library, open the existing song or import a separate copy with its own notes and map. | rare | `managed_duplicate_open`, `managed_duplicate_import`, `import_separate` |
| B4 | Import by dropping a single audio file onto the Library or any view; several files or an unsupported file are refused with a short explanation. | song | — (drop, new) |

### C. Look after a song

| # | Story | Freq. | Actions |
| --- | --- | --- | --- |
| C1 | Correct the title, artist and album read from the file's tags; see the file location and dates without being able to change them. | song | `library_edit`, `library_save`, `library_cancel` |
| C2 | Replace a song's recording (a better take, a moved or changed file), keeping my sections and map when the timing is identical, or clearing them when it isn't. | rare | `library_replace`, `replace_recording`, `replacement_same_timing`, `replacement_new_timeline`, `replacement_cancel` |
| C3 | Delete a song permanently after confirming: its recording copy, notes, map and stems; the original file stays. | rare | `managed_delete`, `managed_confirm_delete` |
| C4 | When opening a song whose recording is missing or has changed, be told what happened and offered Replace recording. | rare | `library_replace` |

### D. Look after the library (in app-wide Settings)

These stories live in a Library section of the app-wide Settings
introduced in the [Stems review](stems.md#design-decisions), not on this
screen. They're listed here so their actions stay covered.

| # | Story | Freq. | Actions |
| --- | --- | --- | --- |
| D1 | See which library is open and where it lives. | rare | `managed_settings` |
| D2 | Switch to another library or create a new one; my pending edits are saved first, and the current song is closed. | rare | `managed_settings`, `managed_create`, `managed_open`, `managed_close` |
| D3 | Know how to move or back up a library: close the app, copy the whole folder, then open or locate it. | rare | — |

### E. Recover

| # | Story | Freq. | Actions |
| --- | --- | --- | --- |
| E1 | When the library folder is missing (unplugged drive, moved folder), see it here and retry, locate it, or open another one; Locate and Open use the same flow as Settings. | rare | `managed_retry`, `managed_locate`, `managed_open` |
| E2 | When the song list fails to load, see why and reload it. | rare | `library_retry` |

### F. Move on

| # | Story | Freq. | Actions |
| --- | --- | --- | --- |
| F1 | Reach the Library from any view, and leave it for the current song at any time. | visit | `show_library`, `library_close` |

## States to design

- **Library empty:** no library yet, or a library with no songs. This is
  also the first-launch screen.
- **Loading** the song list.
- **List:** a handful of songs, and a large library (500+ songs); the
  current song marked; sorted by each column.
- **Search with no results.**
- **Song selected:** its details (album, file location, dates) shown;
  editing; a save that fails (pending edits kept).
- **Import:** the one-time explanation, copy progress, the duplicate
  choice, an import that fails; a file dragged over the window, and a drop
  that's refused.
- **Recording replacement:** same timing or new timing.
- **Delete confirmation.**
- **Library unavailable** (missing folder) and **list failed to load.**
- **Settings, Library section** (designed with app-wide Settings): the
  open library and its location; create, open, locate.
- **Window sizes:** 1180 × 810 default, very large, narrow (~650 px).
- Light and dark appearance.

## Fixed constraints

- One library is open at a time. Songs live in it as copies; imports and
  deletions never touch the original files.
- Opening the Library pauses playback. Opening a song enters Practice paused.
- Import handles one file at a time (by picker or drop), and the copy can't be cancelled once it
  starts. Bulk and folder import, artwork, playlists, Trash and Restore are
  out of scope.
- Title is required; artist and album may be empty. File location, date
  added and last opened are read-only.
- One recording per song. Replacing with unchanged timing requires an
  identical sample rate and length; changed timing clears sections, map,
  loop and saved position, keeping notes, details and controls.
- Deletion is permanent and always confirmed.
- Switching libraries saves pending edits first; a failed save blocks the
  switch. A missing library is never silently replaced by an empty one.
- Moving a library happens outside the app (copy the whole folder).
- Every command comes from the shared action model, with keyboard access to
  search, the list and opening a song.
- Practice decisions carry over: the Stage shell with views as a segmented
  control and "Library ›" in the breadcrumb, toasts for notices, rounded
  rectangles throughout, and the
  [Foundations](README.md#foundations-that-carry-over).

## Open questions for the design

1. **Place in the shell:** the Stage breadcrumb puts Library above the song
   views. Should Library stay a full screen that replaces them, or become a
   quick switcher (a searchable overlay, e.g. on ⌘O or ⌘P) for the common
   "open another song" task, with the full screen for the rest?
-- since the library includes some editor functionality (song metadata, import flow) I think we'll need a full screen here
2. **List shape:** a sortable table with six columns (today), or song rows
   with title over artist · album, and file location and dates moved to
   the details?
-- I'd show title, artist and length in the list. The rest can be moved to the details.
3. **Song status at a glance:** should each row show what the song has
   (measure map, sections, notes, stems) or how far I've got, or keep the
   list to names? This needs new catalogue data.
-- At some point we might add an optional self-review score, but this is a future improvement
4. **Editing details:** a side panel beside the list (today), inline editing
   in the row, or a dialog?
-- Not sure, let's explore options
5. **Replace and delete:** only inside the details (today), or also in a
   per-row ⋯ menu?
-- details only
6. **First launch:** what does a new player see first: the empty Library
   with Import, or a short welcome that leads to it?
-- empty library. Onboarding is a future improvement.
7. **Drag and drop:** accept a single audio file dropped onto the window as
   an import?
-- yes

## Design decisions

Decided during the story review (2026-11-18). The open questions above keep
the maintainer's answers.

1. **No demo:** the built-in demo (a 12-second synthetic chord loop with
   four 3-second sections) is removed from the app. It shows little of what
   the app is for and can't have stems or a meaningful map; importing one's
   own song is the first step. `load_demo` and "Try the demo" go; the
   engine's demo signal stays for the CLI, tests and development fixtures.
   Tracked in [#16](https://github.com/zantoku/OhMyLoop/issues/16).
2. **Library folder in Settings:** seeing, creating, opening and switching
   the library, and the advice on moving it (D1–D3), move from the library
   dialog to a Library section of the app-wide Settings. The Library screen
   keeps only the recovery notices (E1, E2); the first-import setup (B2)
   stays in the import flow.
3. **Full screen:** Library stays a full screen in the shell, not a quick
   switcher, because it also holds editing (song details) and the import
   flow.
4. **List:** each row shows title, artist and length. Album, file location
   and dates move to the song's details. Search still matches title,
   artist, album and file location. Length is new in the catalogue; the
   song record already knows it.
5. **No song status in the list.** An optional self-review score may come
   later; it's out of scope for this pass.
6. **Editing details:** a side panel beside the list, opened only by an
   explicit edit action (concept LD, chosen 2026-11-18). See
   [Chosen concept](#chosen-concept-open-without-editing).
7. **Replace recording and delete** stay inside the song's details only, no
   row menu.
8. **First launch** shows the empty Library with Import. Onboarding is a
   later improvement.
9. **Drag and drop:** dropping one audio file onto the window imports it,
   through the same flow as Import (B4). Several files or unsupported types
   are refused with an explanation.

## Chosen concept: open without editing

Four layouts were drawn: a side panel that opens on selection (LA), editing
in place (LB), a details dialog (LC), and, after reviewing those, LD. LD was
chosen on 2026-11-18. Its rule: **opening a song never shows its edit
fields;** editing needs an explicit edit action.

- **Rows** show title, artist and length. The loaded song has a blue bar
  and a CURRENT tag.
- **Mouse:** hovering a row shows → before the title and ⋯ at the end of the
  row. → opens the song in Practice, paused (A4); it's an arrow, not ▶,
  because playback doesn't start. Double-click opens too. A single click
  only selects the row; arrow keys move the selection and Enter opens it.
- **Back to the current song** (A5, F1): the loaded song always shows its →,
  which returns to Practice as I left it, without reloading
  (`library_close`). Esc does the same. There's no separate "Back to …"
  button in the header.
- **Editing:** ⋯ or ⌘I opens the editor panel beside the list: title,
  artist and album fields; length, dates and file as read-only facts; and,
  at the bottom, Replace recording and Delete song (C1–C3).
- **Touch:** a tap on the row opens the song. → and ⋯ are visible on every
  row when there's no hover (a coarse-pointer media query); long-press
  opens the editor. Rows are 46 px tall.
- **Header:** "Library" as the breadcrumb root, ✓ Saved, a labelled
  Settings button and the keyboard-shortcuts button. Library is the home
  of the app-wide Settings button; ⌘, opens Settings from any screen, and
  on macOS the app menu has "Settings…".
- **Appearance:** the theme choice moves from the status bar into Settings
  → Appearance.
- Search, sort, Import, the drop target, and the empty, loading, no-match,
  missing-library, failed-list, import and refused-drop states are shared
  with the other concepts.

Still to design: a large library (500+ songs), narrow window, light
appearance.
