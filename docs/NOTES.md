# Song notes

Implemented: 2026-10-11; redesigned as a drawer 2026-11-19 (concept NC in
[docs/design/notes.md](design/notes.md)). Plain-text notes belong to the stable
UUID song record, independently of its reference timeline or audio file.

## Workflow

Notes is a 320 px drawer on the right edge of Practice, from below the header
down to the playback bar, beside both the readout band and the waveform. The
readout band and waveform narrow while it's open; the sections sidebar is
unaffected. Notes is not a view: the header's Notes button and **N** toggle the
drawer, and it has no close X. From Measure map or Stems the Notes button opens
Practice with the drawer. The drawer stays open across views and songs until
closed. Opening or closing it never changes the transport.

Opening the drawer focuses the text for writing; closing it returns focus to
where it was, or to the waveform. The text area owns every key while focused,
so Space and letter shortcuts never fire while typing. **Esc** returns focus to
the waveform, after which Space plays again; the drawer's hint says so. The
transport stays clickable throughout, and notes stay editable while separation
runs. The editor preserves text, whitespace and line breaks without Markdown
rendering, with spell checking, an empty-notes placeholder, and the native text
context menu, Undo/Redo, Cut/Copy/Paste and Select All.

Edits autosave after 300 ms, coalescing rapid input. Closing the drawer, opening
Library, changing songs and normal window/app exit flush pending edits. A failed
write keeps the latest text, shows an alert with Retry save in the drawer and
"⚠ Notes not saved" in the header, and prevents those transitions until saved; a
blocked Library, song change or quit brings Practice forward with the drawer.
The text is read-only while the app is busy (opening, importing, restoring), so
nothing typed then can be lost when the song changes. Notes over 1 MiB of UTF-8 are kept in the drawer but not sent;
the last saved version stays on disk and an alert asks to shorten them. Edits
made during a write are saved again after that write completes. The header's
save state covers normal saving; the drawer shows "Saving…" only when a save
takes longer than 800 ms. Forced termination/crashes can lose text not yet
persisted; this is not a recovery journal.

The notes font is the proportional UI font by default. `ui/notes-font.ts` reads a
monospace preference that Settings → Appearance will write (Settings decision 5);
until Settings exists nothing sets it.

## Architecture and persistence

- `ui/NotesDrawer.svelte` presents the drawer: text area, slow-save status,
  failure and size alerts, and the Esc hint. `PracticeScreen` renders it at its
  right edge through a `drawer` snippet; App owns whether it is open.
- `ui/notes-session.ts` owns debounce, pending text, the 1 MiB check, slow-save
  detection, serialized flush/retry and disposal guards; App connects it to the
  existing document-save queue.
- `ui/notes-font.ts` is the notes font preference seam.
- `ui/actions.ts` centralizes the `show_notes` (toggle, N), `close_notes` and
  `retry_notes` identities, button bindings and descriptions.
- `src/project.rs` validates and stores song-level `notes` in the atomic version-2
  `<practice-directory>/<song-uuid>.json` envelope. Replacing the recording
  keeps the notes, while intentional separate imports get their own notes.
  No audio tags are modified.
- The UI practice DTO is version 5. Recognized pre-UUID metadata is reset once on
  launch rather than migrated; malformed/future UUID data remains preserved.
  Notes are bounded to 1 MiB of UTF-8 text; oversized saves preserve the prior file.
  See [UUID persistence/reset](UUID_IDENTITY.md).
- `src-tauri/src/lifecycle.rs` intercepts normal close/quit, asks the UI to flush,
  and exits after acknowledgment. Existing native shutdown still saves position
  and stops audio/analysis. No file operations enter the audio callback.

Tests cover whitespace round trips, recording replacement, invalid-save
preservation, coalescing, edits during pending writes, retry, disposal, shortcut
isolation, navigation and quit flushing. Earlier native macOS review used isolated app data and generated audio. This UUID
change has automated macOS checks; Linux and Windows are not locally verified.

Markdown, rich text and notes export remain outside this pass.
