# Managed music library

Implemented issue #3 on 2026-10-13. The demo works without setup. **Import song**
asks for a library on the first import, then remembers it. Create library chooses
a parent and creates an **OhMyLoop Library** subfolder; it never overwrites an
existing folder. **Open existing library** selects an existing library root.
Both actions are also in Library settings.

## Import and practice

Imports copy recordings directly to their final managed location and leave
external originals untouched. The UI explains this once. File-picker dismissal
cancels before selection; after selection there is copy progress and ordinary
loading, with no cancellation or separate verification step. Copying accumulates
the stored SHA-256 digest from the bytes actually written. Decoding must succeed
before publication. Observed failures report an error and attempt immediate
cleanup; cleanup failures are included in the error.

Exact-copy detection offers **Open existing song** or **Import separate song**.
Each separate import owns a physical copy and independent UUID, notes and map.
Successful imports and library opens enter Practice paused. Search and sorting
retain title, artist, album, file location, date added and last opened. Initial
metadata comes from tags with filename fallback; **Edit → Save details** persists
user overrides in the song document. File paths and dates are read-only.

A song has one reference recording/map. **Replace recording…** appears only below
File location in the Library edit panel. The leading X closes its dialog.
Explicit unchanged-timing replacement requires identical decoded rate/frame count
and preserves positional annotations. Changed timing clears sections, map, loop
and saved position; UUID, notes, metadata and controls survive. After the new
reference saves, the previous managed recording is removed. Cleanup failures are
reported. There is no retained recording history.

Opening a song hashes its managed recording outside the audio callback. Missing
recordings and unexpected content changes are reported; changed audio requires
explicit replacement before playback. There is no background watcher.

**Delete song** appears only in the Library edit panel. Its confirmation explains
permanent deletion of the song's recording, notes, map and any future stem assets.
External originals remain untouched. Active deletion stops and unloads the song
and stays on Library. There is no Trash or Restore.

## Library selection and portability

Switching libraries saves pending practice, notes and metadata edits, then stops
playback and unloads the song. The new library opens without selecting a song.
A failed switch leaves the previous selection; a failed pending save blocks it.
Normal restart restores the last song using both the selected library UUID and
song UUID. A missing selected root offers Retry, Locate library and Open another
library, while the demo remains available. Locate requires the same library UUID.
No empty library is silently substituted.

To move or back up a library, close the app and copy/move the **whole folder**.
Then open it using Open existing library (or Locate library for the selected
missing library). Relative asset links preserve IDs and annotations. There is no
in-app move command. Any accessible folder can be selected; network/sync storage
is not classified or blocked. Concurrent access and sync conflicts are not handled:
assume one app instance accesses a library at a time.

## Persistence and failure boundaries

```text
<library>/library.json                   # manifest v1, library UUID
<library>/catalogue.json                 # rebuildable catalogue projection
<library>/songs/<song-uuid>/song.json     # authoritative managed envelope v1
<library>/songs/<song-uuid>/assets/<asset-uuid>/<safe-name>
```

The managed envelope owns metadata, dates and a version-2 song record with its
current asset/timeline, notes and controls. App data retains `managed-library.json`
(version 1: selected root/UUID, last song and import explanation preference),
preferences and model caches. Existing external-link UUID records are neither
migrated nor deleted; import recordings afresh. The old pre-UUID reset remains
narrowly scoped as documented in [UUID identity](UUID_IDENTITY.md).

The list is reconstructed from authoritative song documents. Catalogue writes
are projections; failures after a song saves are reported accurately. JSON saves
use individual atomic replacements. Invalid/future documents and preferences are
preserved, and relative owned paths reject traversal and symlinks.

There is **no journal, restart recovery, locking, staging, trash or storage-size UI**.
Unexpected termination can leave partial imports, unreferenced files or partial
deletion. There is no automatic startup cleanup. Atomic JSON files do not imply
multi-file transactions or guaranteed durability on every filesystem.

## Architecture and verification

- `src/managed.rs` and `src/managed/`: schemas, preferences, path policy and direct transfers.
- `src/desktop/managed.rs`: worker preparation and controller commit orchestration.
- `src/library.rs`: shared metadata/catalogue DTOs and tag probing.
- `ui/managed-library-session.ts`: setup, settings, import and duplicate dialog state.
- `ui/LibraryScreen.svelte`: catalogue, metadata edit and sole deletion entry point.
- Tauri blocking tasks run copy/hash/decode outside the desktop control thread.

Generated native fixtures cover copies, independent duplicates, failure cleanup,
relative moves, metadata, replacements, hash changes, schema/path/symlink rejection,
stale commits, restart and deletion. Frontend checks cover setup/settings,
duplicate choices, progress, pending-save failures, deletion confirmation and
disposed replies. Automated checks run on macOS; this change has no new hardware,
subjective listening, Windows or Linux evidence.

Deferred: bulk/folder imports, artwork, playlists, live sync, recovery and managed
stems/inference (later issues). See [the agreed plan](MANAGED_LIBRARY_IMPLEMENTATION_PLAN.md).
