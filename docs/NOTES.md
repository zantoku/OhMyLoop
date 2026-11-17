# Song notes

Implemented: 2026-10-11. Plain-text notes belong to the stable UUID song record,
independently of its reference timeline or audio file.

## Workflow

Notes sits left of Measure map above the practice waveform, enabled with a loaded
song. The editor preserves text, whitespace and line breaks without Markdown
rendering. The X beside its title inside the white panel saves and returns to practice. Measure map uses the same
X placement; both keep Library at the top right. Library itself retains Practice
at the top right. Switching between notes and practice does not change transport.
Opening Library pauses playback after pending notes have saved.

Edits autosave after 300 ms, coalescing rapid input. Closing notes, opening Library,
changing songs and normal window/app exit flush pending edits. A failed write keeps
the latest text for Retry save and prevents those transitions until saved. Edits
made during a write are saved again after that write completes. The UI shows
Saving, Unsaved changes or Saved. Forced termination/crashes can lose text not yet
persisted; this is not a recovery journal.

## Architecture and persistence

- `ui/NotesScreen.svelte` presents the textarea and status.
- `ui/notes-session.ts` owns debounce, pending text, serialized flush/retry and
  disposal guards; App connects it to the existing document-save queue.
- `ui/actions.ts` centralizes notes/close/retry identities, button bindings and
  descriptions. Notes has its own action scope and typing suppresses shortcuts.
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
