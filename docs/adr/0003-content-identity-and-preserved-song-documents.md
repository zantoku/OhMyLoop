---
status: accepted
identity_superseded_by: ADR-0005
---

# Key song documents by file content and preserve unreadable user data

The content-based **song identity** decision is superseded by
[ADR 0005](0005-stable-song-identity-and-audio-assets.md) on 2026-10-12.
Version validation, supported migration and preservation of unreadable user
data remain accepted. The implementation below is the historical/current
baseline until the UUID migration is delivered.

Retrospectively documented on 2026-10-12. These choices were implemented in
desktop commit `b5bf140` (2026-10-08); their original decision date is not recorded.

Use the complete source file's SHA-256 digest as the identity shared by desktop
practice documents, library entries and analysis. Persist practice data in
versioned JSON documents, validate at load/update boundaries, migrate supported
older versions and report malformed or unsupported future files while preserving
them. Saves validate before temporary-file write, file sync and replacement.

The identity implementation explicitly states the rationale: moving or renaming
an unchanged file retains its practice data, while replacement audio at the same
path must not inherit annotations. Preserving invalid files protects user changes
when this application cannot safely interpret them. These guarantees carry
meaningful costs if identity or persistence is later changed.

Byte-identical copies share identity. Retagging or re-encoding changes identity
even when the recording sounds the same; the current workflow has no automatic
annotation transfer to changed audio. Full-file hashing adds work on opening a
song, outside playback processing. Temporary-file replacement is not a backup
strategy or proof of power-loss durability on every filesystem. No historical
comparison with perceptual fingerprints or a database was recovered.

## Evidence

- [Desktop persistence](../DESKTOP.md#boundaries) records content keys, atomic
  replacement and preservation in both the original commit and current docs.
- [Song identity and store](../../src/project.rs) implements hashing, migration,
  validation and saving; [storage](../../src/storage.rs) implements replacement.
- [Library](../LIBRARY.md) documents complete-file identity, changed-file behavior
  and preservation of invalid catalogues.
