# Managed library implementation plan

Scope agreed 2026-10-13 for [issue #3](https://github.com/zantoku/OhMyLoop/issues/3),
following completion of UUID identity in #2. Implementation is complete and undergoing review.
This replaces the broader 2026-10-12 proposal.

## Agreed product behavior

- The demo works without library setup. On the first import, select or create a
  library and remember it. Creation makes an **OhMyLoop Library** subfolder in the
  chosen parent; opening an existing library selects its root directly. Never
  overwrite a pre-existing folder when creating a new library.
- **Import song** replaces the file-picker action called Open song. Explain once
  that recordings are copied and external originals remain untouched. Successful
  import opens the song in Practice, paused.
- Selecting the source file commits to importing. Copy directly to the intended
  managed asset location, with copy progress but no subsequent cancellation,
  staging directory or separate verification phase. File-picker dismissal before
  selection still cancels. Ordinary decoding must succeed before the song is
  playable; this is not a separate import-verification workflow.
- Exact-copy detection offers **Open existing song** or **Import separate song**.
  Never merge automatically. Choosing Open existing does not create another song.
  Each independent import has its own UUID, notes/map and physical recording copy.
  The duplicate choice is the agreed exception to immediately creating a new song
  after file selection; no additional generic import confirmation is introduced.
- Each song has one current recording and map. Timing-compatible replacement
  requires explicit unchanged-timing confirmation and identical decoded rate/frame
  count. Changed timing clears sections, map, loop and saved position. Song UUID,
  notes, metadata and controls survive. No retained recording/map history.
- Store a content digest on import and check it when opening the song, outside the
  audio callback. A missing or changed recording is reported; a changed recording
  requires explicit replacement before playback. No background file watcher.
- **Delete song** appears only in the Library edit panel. Confirmation permanently
  removes the song's managed recording, notes, map and any stems. Stop playback
  and unload the song if it was active; stay on the Library screen without loading
  another song or the demo. External originals remain untouched. No Trash/Restore.
- **Create library** and **Open existing library** live in Library settings.
  Switching saves pending edits, stops playback and unloads the current song. Open
  the new library without selecting a song. A failed switch keeps the previous
  library selected. Pending-save failure must prevent switching and losing edits.
- An unavailable selected root offers **Retry**, **Locate library** and **Open
  another library**, with the demo still available. Never silently substitute an
  empty library. Locate library checks its library UUID.
- Users may select any accessible folder. No detection/blocking of network drives
  or sync folders. Report concrete errors and actual tested environments. Assume
  one app instance accesses a library at a time; no locking or instance detection.
- Move the complete folder with the app closed, then open it at its new location.
  Relative links preserve all IDs and annotations. No in-app Move Library command.
- Failed copy/decode reports an error, leaves the song out of the catalogue and
  attempts to remove the copied file. No automatic retry or cleanup on restart.

## Explicit exclusions

No journal, restart recovery, staged imports, separate copy verification,
post-selection import cancellation, writer locks, read-only mode, recording
history, trash/restore, storage-size displays, disk-space preflight, automatic
relocation or bulk/folder import. Stems and inference are later deliverables.
Concurrent access and cloud-sync conflicts are not handled by this slice.

Existing external-link UUID records are not automatically imported or deleted.
Import audio afresh. The previous pre-UUID reset waiver does not authorize another
wipe of UUID data. Preserve old app-data records outside the managed catalogue.

## Storage and architecture

Use a library UUID and library-relative audio paths. Keep the current embedded
reference timeline in the authoritative song document; move metadata and dates
there so a catalogue projection is rebuildable without losing user edits.
Validate and version schemas independently of existing app-data records.

```text
<library>/
  library.json                     # library UUID/schema
  catalogue.json                   # list projection
  songs/<song-uuid>/song.json       # metadata, current asset/timeline, notes, controls
  songs/<song-uuid>/assets/<asset-uuid>/<safe-name>
  songs/<song-uuid>/analysis/...     # derived timing evidence
```

App data retains active-library and last-song pointers, preferences and replaceable
model/runtime caches. Last-song restoration checks library UUID and song UUID.
Root switching opens without selecting a song; ordinary application restart keeps
existing last-song restoration behavior. The demo remains transient.

A cohesive managed-library module owns create/open, import, replacement, song
updates and deletion. Keep filesystem policy out of UI handlers and playback DSP.
Reuse atomic JSON document replacement, existing decode/tag probing and source
revision/session guards. Atomic individual documents do not make a multi-file
import or deletion transactional.

Reject absolute/traversing stored asset paths and symlink escapes. Delete only
paths owned by the selected library/song. Accept an import-source path as read-only
provenance, never as a deletion target. Do not add storage-type restrictions.

For replacement, copy to a new asset's final location and decode before updating
its song reference. Keep the current recording/map until that update succeeds;
then remove the unreferenced previous recording. Report cleanup failures. There
is no historical map or recording restore UI.

Source identity may be hashed for duplicate detection and recorded provenance;
there is no independent post-copy source-vs-destination verification pass.
Checking stored audio on later song open remains in scope. Copy/decode work runs
outside the callback and without blocking the desktop control thread.

## Failure boundaries

Report copy, decode, schema and save failures without altering external originals.
Attempt immediate cleanup of failed import output. Preserve malformed/future user
records. A failed catalogue projection update after a successfully saved song
must be reported accurately rather than described as a failed audio import.

Without journaling/recovery, unexpected termination can leave partial imports,
unreferenced files or partially completed deletion. There is no restart repair,
deferred cleanup queue or recovery UI in #3. Ordinary observed errors still need
useful messages; do not imply multi-file all-or-nothing guarantees.

## Implementation order and acceptance

1. Root manifest, managed song schema, relative-path validation and atomic saves.
2. Direct-copy imports, duplicate choices, decode/activation and replacement.
3. Library settings/setup, safe switching, unavailable-root actions and progress.
4. Confirmed deletion, missing/changed recording status and manual-move/backup docs.
5. Focused regression checks and standards/spec review.

Generated audio and temporary libraries should verify independent duplicates,
source immutability, direct-copy failure cleanup, decode failure, relative links,
metadata persistence, replacement preservation/reset and stale-reference guards.
Cover invalid/future schemas, unsafe paths/symlinks, reported permission/copy/save
failures, confirmed deletion and reopening a manually moved library with unchanged
IDs. No journal/recovery/lock/trash tests or implementations are required.

Frontend checks cover first-import setup, picker dismissal, duplicate choices,
copy progress without cancel/verification UI, Library settings, root-switch save
failure, unavailable roots, edit-only deletion confirmation and disposed replies.
Run formatting, strict Clippy, native/frontend focused regression tests, Svelte
checks and build. Report actual platform/hardware evidence, with untested platforms
explicitly identified. Document closed-app whole-folder backup/move/reopen.
