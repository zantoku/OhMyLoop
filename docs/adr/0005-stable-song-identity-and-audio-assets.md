---
status: accepted
---

# Give songs stable identities independent of their audio files

Decision: 2026-10-12. Assign a UUID when a song is imported and store its audio
file links as data. A song is a practice item, not a file's byte contents:
retagging, volume changes, relocation and generating stems must not create a new
identity or detach its notes. Hashes remain useful for integrity, exact-copy
detection and inference provenance, but do not identify songs.

This supersedes the **identity portion** of
[ADR 0003](0003-content-identity-and-preserved-song-documents.md). Its requirements
to validate versioned documents, migrate supported versions and preserve unreadable
user data remain accepted. [ADR 0001](0001-source-recording-coordinates.md) still
governs annotations; stable song identity does not establish audio alignment.

For this pre-release transition, the user explicitly waived legacy-data
preservation on 2026-10-12: reset incompatible content-keyed metadata on launch
instead of migrating it. This is a scoped exception to ADR 0003, not a policy
of deleting future unreadable data.

## Consequences

- Audio assets have their own IDs and immutable content revisions. Replacing a
  recording changes the selected asset, not the song UUID. Keep previous assets
  until explicitly removed.
- Exact-copy detection offers opening the existing song or importing a separate
  song. Neither filenames nor hashes silently merge user records. Intentional
  duplicates can have independent notes and maps.
- A changed file at a known location is an integrity/replacement problem for
  that asset. It does not become a different song automatically. Managed audio
  is immutable through the app; replacements are imported as new assets.
- Reopening and late replies use song UUID plus session/job generation and asset
  or timeline revision as appropriate. UUID alone is insufficient to reject work
  from an older recording belonging to the same song.
- Reset the known legacy catalogue, practice documents, last-song state and
  derived timing caches before loading them. Existing audio and prototype stem
  outputs remain untouched; songs receive UUIDs when imported afresh. Make reset
  restart-safe and schema-gated so it cannot erase newly created UUID records.

Perceptual matching was rejected as the identity mechanism: it can help future
discovery, but should not decide whether two practice items share user data.
