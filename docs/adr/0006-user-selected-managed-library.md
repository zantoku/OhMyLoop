---
status: proposed
---

# Store imported recordings and stems in a user-selected managed library

Proposed: 2026-10-12, following the user's preferred direction. Copy imports into
a selected library root and keep song documents, audio assets and completed stem
sets there. This replaces the current external-file catalogue for new imports.
It costs disk space, but gives the app ownership of generated files and makes a
song's recordings and practice data relocatable together.

Store library-relative paths and a library UUID. Keep app preferences, device
settings and replaceable model/runtime caches in app data. A relocated library
is selected again by its manifest; relocation must not change song or asset IDs.
The initial implementation supports one active library and one writing app
instance, with locking and recoverable staged operations. Do not imply that
ordinary JSON replacement makes multi-file imports transactional.

## Consequences

- Validate and copy to staging, verify the copied bytes, then publish immutable
  assets and commit their references. A recovery journal distinguishes incomplete
  operations from completed data; rebuildable catalogue views are not the sole
  authority for song ownership.
- The import source path is provenance, never the playback dependency after a
  successful copy. The app does not move, edit or delete the external original.
- For this pre-release transition, reset incompatible app metadata on launch and
  start with an empty catalogue, as authorized by the sole user. Import existing
  audio afresh; do not copy it automatically, delete it or retain a legacy-link
  compatibility layer. The reset scope is recorded in ADR 0005 and the plan.
- Root changes select/open a library; moving an existing library is a separate
  verified copy-and-switch operation. Keep the old root until success and explicit
  removal. An unavailable root is an error, not permission to create an empty
  replacement library or silently use another folder.
- Removal and disk usage are explicit workflows. Delete only app-owned paths
  inside this library after checking references and active jobs. Keep user data
  recoverable through a library trash area before final deletion.

External references for all new imports were considered: cheaper in space, but
they retain missing/moved-file problems and split ownership between source files
and generated stems. Cloud sync and concurrent writers are separate future work.
