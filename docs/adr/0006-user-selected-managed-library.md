---
status: accepted
---

# Store imported recordings and stems in a user-selected managed library

Decision updated: 2026-10-13 after the product-owner scope discussion for #3.
Copy imports into a user-selected library so recordings and practice data can
move together, independently of external originals. Persist library-relative
paths and stable library/song/asset/timeline UUIDs; app preferences and replaceable
model/runtime caches remain in app data.

The first import prompts for library setup; the demo does not require it. Creation
makes an OhMyLoop Library subfolder. Library settings provide creation/opening;
manual moves happen with the app closed. Missing roots report an error rather
than silently creating a replacement. Any accessible folder may be selected.

## Deliberate scope limits

The user chose a small direct-copy implementation over the earlier staged,
journaled proposal. Selecting a file commits to the import, subject to the agreed
exact-duplicate choice. Copy to the final asset location with progress and no
cancellation or separate verification phase. Decode for ordinary playback;
failed imports report errors and attempt immediate output cleanup. Atomic JSON
saves remain in scope. No journal or restart recovery is implemented, so unexpected
termination can leave incomplete/unreferenced files or partial deletion.

Assume one instance accesses a library at a time; do not implement locking or
second-instance detection. Network/sync paths are not actively detected or blocked.
No in-app relocation, storage-size displays or disk-space preflight in #3.

Delete songs permanently after confirmation from the Library edit panel. There
is no Trash/Restore. One song has one recording/map; replacement removes the old
recording after successfully saving its new reference, with no retained history.
External originals are never edited, moved or deleted. Check managed recording
content digests on song open and require explicit replacement for changed audio;
no background watcher. Guard owned paths and preserve malformed/future user data.

This revises the earlier proposal for locking, staged verification, recovery,
trash and verified in-app relocation. See the
[agreed implementation plan](../MANAGED_LIBRARY_IMPLEMENTATION_PLAN.md) for UI,
error handling and acceptance criteria. The earlier reset waiver remains limited
to recognized pre-UUID metadata; #3 does not delete existing UUID app-data records.
