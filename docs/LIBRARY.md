# Music library: first implementation pass

Implemented 2026-10-11, with UUID identity/replacement/reset added 2026-10-12.
Audio remains at its external location until managed copying/root selection in
issue #3. [UUID identity](UUID_IDENTITY.md) documents this slice's exact storage
and reset boundaries; [Managed songs and stems](STEMS.md) describes later work.

## Workflow

- Open **Library** from the app header. Entering it pauses current playback.
  The same header button becomes **Practice** to return. **Open song** lives in the
  library panel heading, including when the catalogue is empty. Cmd/Ctrl+O opens
  the library before showing the file picker; cancellation leaves the library
  visible and keeps the current song.
- Successfully opening a real song adds it automatically. The demo is excluded.
  New imports receive stable UUIDs; last-song restoration uses that UUID. The demo
  remains transient. There is no historical directory scan or legacy migration.
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
  Ties use title and song UUID for deterministic ordering.
- Click a title to reopen its practice state, paused. Missing files and changed
  content at the stored location report an error while retaining the current song.
  Use **Open song** to locate a moved unchanged file; its existing entry and
  annotations are reused and its canonical location is updated. **Import separate
  song** intentionally creates independent notes/maps for identical audio.
- **Replace recording** is available for the loaded song and each library row,
  including changed/missing files after restart. Choose unchanged timing explicitly
  (same decoded rate/frame count required) or a new retained timeline. Notes and
  controls stay with the song UUID; previous recordings/maps remain available.
  [Reference handling](UUID_IDENTITY.md) describes retained selection and limits.
- **Edit → Save details** edits title, artist and album in the library. Title is
  required. Fields are trimmed and bounded to 1024 UTF-8 bytes each. Opening the
  file again preserves these overrides. File location and dates are read-only.
  Failed saves retain typed edits and the previous catalogue. Cancel discards
  form edits; successful title changes also update the active song header.

Search, sort direction and the editor are UI-session state. Switching back to
practice retains the loaded song; opening a different song uses the existing
practice flow and resets its single-song mapping session as before.

## Persistence and identity

`library.json` sits **inside the practice directory**, alongside `<song-uuid>.json`
and `last-song.json`. Normally this is the per-user app data directory's `practice`
folder; `OHMYLOOP_DATA_DIR` redirects the entire practice directory, including the
catalogue. Back up this file along with practice annotations, analysis and audio.
The library index is separate from the version-1 song envelope, which embeds
versioned song-level data and reference-timeline records. See [the schema](UUID_IDENTITY.md).

Version-2 shape:

```json
{
  "version": 2,
  "songs": [{
    "song_key": "<canonical song UUID>",
    "metadata": {"artist": "", "title": "Song title", "album": ""},
    "file_location": "/canonical/path/song.wav",
    "date_added": 1791700000000,
    "last_opened": 1791700000000
  }]
}
```

One entry per song UUID. Exact content hashes identify audio revisions and suggest
reopening existing recordings; **Import separate song** never merges them.
Known-path integrity changes are reported and require explicit recording replacement.
The authorized pre-release launch reset removes only recognized old metadata, before
restoration, and leaves audio/preferences/models/prototype outputs intact. It does
not migrate annotations; malformed/future UUID data is preserved.

Dates are UTC milliseconds since Unix epoch, displayed in local time. Date added
is the first successful library registration. Last opened updates on a successful
open, including last-song restoration; metadata edits alone do not change it.
A backwards system clock cannot reduce recorded last opened timestamps.

Rust validates version, unique identities, metadata, locations and dates, with
10,000 songs/16 MiB bounds. Invalid, unreadable and future-version catalogues are
reported and left intact. A corrupt catalogue is preserved and reported. Valid UUID last-song restoration
and practice remain available; index lookup/edit operations still report the error. Writes use the
same temporary-file, sync and rename helper as practice documents. There is no
saved library history, automatic backup, or migration from future versions.

## Architecture and verification

- `src/library.rs`: versioned DTOs, validation and catalogue store.
- `src/library/tags.rs`: initial metadata probe, outside playback DSP/callbacks.
- `src/storage.rs`: shared durable JSON file replacement.
- `Desktop`: serializes registration, catalogue reads/edits and reopening on the
  control thread; it verifies asset integrity and reference revisions before replacing the active song.
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
