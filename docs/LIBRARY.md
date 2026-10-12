# Music library: first implementation pass

Implemented 2026-10-11. A catalogue of local practice songs; audio stays at its
original location, with existing content-keyed practice and analysis data.

The next integration is planned in [Managed songs and stems](STEMS.md).
[ADR 0005](adr/0005-stable-song-identity-and-audio-assets.md) replaces hash-based
song identity with UUIDs; managed imports and migration are planned, not yet
implemented. This document describes the currently shipped catalogue behavior.

## Workflow

- Open **Library** from the app header. Entering it pauses current playback.
  The same header button becomes **Practice** to return. **Open song** lives in the
  library panel heading, including when the catalogue is empty. Cmd/Ctrl+O opens
  the library before showing the file picker; cancellation leaves the library
  visible and keeps the current song.
- Successfully opening a real song adds it automatically. The demo is excluded.
  Existing songs with older practice documents join when next opened, including
  automatic restoration of the last song. There is no historical directory scan.
- Read initial artist, title and album using Symphonia's metadata probe. Untagged
  titles use the file stem; missing artist/album stay empty. WAV RIFF text
  terminators are stripped. Import is best-effort; unsupported tags use fallback
  metadata. No embedded tags are written.
- Search title, artist, album and file location. All query words must match across
  these fields, without case sensitivity. Dates are sorting criteria, not search.
- Sort by Title, Artist, Album, File location, Date added or Last opened, using
  column buttons or the Sort by selector. Click the current column or the direction
  button to reverse. Default: Last opened, descending. Text uses locale-aware,
  case-insensitive numeric ordering; blank metadata stays last in both directions.
  Ties use title and content identity for deterministic ordering.
- Click a title to reopen its practice state, paused. Missing files and changed
  content at the stored location report an error while retaining the current song.
  Use **Open song** to locate a moved unchanged file; its existing entry and
  annotations are reused and its canonical location is updated.
- **Edit → Save details** edits title, artist and album in the library. Title is
  required. Fields are trimmed and bounded to 1024 UTF-8 bytes each. Opening the
  file again preserves these overrides. File location and dates are read-only.
  Failed saves retain typed edits and the previous catalogue. Cancel discards
  form edits; successful title changes also update the active song header.

Search, sort direction and the editor are UI-session state. Switching back to
practice retains the loaded song; opening a different song uses the existing
practice flow and resets its single-song mapping session as before.

## Persistence and identity

`library.json` sits **inside the practice directory**, alongside `<song-hash>.json`
and `last-song.json`. Normally this is the per-user app data directory's `practice`
folder; `OHMYLOOP_DATA_DIR` redirects the entire practice directory, including the
catalogue. Back up this file along with practice annotations, analysis and audio.
The library index remains separate from the version-4 practice document (which
adds song notes).

Version-1 shape:

```json
{
  "version": 1,
  "songs": [{
    "song_key": "<64 hexadecimal SHA-256 characters>",
    "metadata": {"artist": "", "title": "Song title", "album": ""},
    "file_location": "/canonical/path/song.wav",
    "date_added": 1791700000000,
    "last_opened": 1791700000000
  }]
}
```

One entry per complete-file SHA-256 identity, shared with practice documents and
analysis. Opening another byte-identical copy updates the existing location;
it does not create a second entry. Retagging/re-encoding changes file identity and
is treated as a new song. There is no annotation transfer or relink-to-changed-audio
workflow in this pass.

Dates are UTC milliseconds since Unix epoch, displayed in local time. Date added
is the first successful library registration. Last opened updates on a successful
open, including last-song restoration; metadata edits alone do not change it.
A backwards system clock cannot reduce recorded last opened timestamps.

Rust validates version, unique identities, metadata, locations and dates, with
10,000 songs/16 MiB bounds. Invalid, unreadable and future-version catalogues are
reported and left intact. A corrupt catalogue does not prevent opening and editing
practice songs; catalogue registration errors appear in the app. Writes use the
same temporary-file, sync and rename helper as practice documents. There is no
saved library history, automatic backup, or migration from future versions.

## Architecture and verification

- `src/library.rs`: versioned DTOs, validation and catalogue store.
- `src/library/tags.rs`: initial metadata probe, outside playback DSP/callbacks.
- `src/storage.rs`: shared durable JSON file replacement.
- `Desktop`: serializes registration, catalogue reads/edits and reopening on the
  control thread; it verifies file identity before replacing the active song.
- Tauri exposes read/open/update library commands using blocking-task orchestration.
- `ui/library.ts`: DTOs, filtering, ordering and local date formatting.
- `ui/library-session.ts`: persistence orchestration and disposed-reply handling.
- `ui/LibraryScreen.svelte`: presentation and metadata form. Central actions,
  payloads, button bindings and descriptions remain in the keymap/action model.

Generated-data native tests cover tag import/file immutability, durable edits and
dates, moved/missing/replaced files, retained practice annotations and malformed
catalogues. Frontend tests cover all sort fields and directions, search, scoped
bindings, load/save errors, reopening and asynchronous lifecycle handling.
Native UI/platform evidence is recorded in [IMPLEMENTATION_STATUS.md](IMPLEMENTATION_STATUS.md).

Deferred: batch/folder imports, removal, artwork, playlists, alternate locations
for identical copies, library export/sync and portable project packages.
