# UUID song identity (issue #2)

Implemented identity slice of [the stem integration plan](STEMS.md). Managed
library root selection and copying are implemented in issue #3; see [Library](LIBRARY.md). Imported audio is still an
external file link in this slice; neither reset nor replacement deletes it.

A song gets a canonical nonzero UUID. Audio assets and reference timelines have
separate typed UUIDs. Complete-file SHA-256 is an asset integrity digest. Normal
Open song reopens a known path/exact recording; **Import separate song** creates
an independent UUID even for byte-identical audio. Notes and speed/pitch settings
belong to the song. Sections, loops, accepted measure maps and last position belong
to a versioned reference timeline, in the existing decoded source coordinates.

**Replace recording…** is in **Library → Edit**, below File location. A song
has one current recording and one authored map. Timing-compatible replacement
retains them after the user's explicit unchanged-timing assertion and identical
decoded sample rate/frame count checks; equal duration alone is insufficient.
Choosing changed timing clears the map, sections, loop and saved position, as
explained in the dialog. Notes and speed/pitch settings survive either choice.
There is no recording history or switching back to an old map. Changed/missing
external files can be replaced from Library without assigning a new song UUID.

## Persistence and startup

The configured practice-data directory holds:

- `uuid-schema.json`: completed pre-release reset, version 1, `uuid-song-records`.
- `library.json`: catalogue version 2; stable song UUIDs and descriptive metadata.
- `<song-uuid>.json`: atomic song envelope version 2, containing song-level notes/
  controls, one audio asset and one version-1 reference timeline.
- `last-song.json`: version 2, containing a song UUID instead of a path.

The UI's flattened practice-document DTO is version 5, assembled from the song
and selected timeline; it is not the on-disk authoritative schema. Its opaque
reference token binds song, asset, timeline, reference revision and a fresh session
UUID. Native mutations and UI asynchronous replies reject older assets and sessions,
including reopening the same song. Analysis candidates are version 2 with an
explicit `reference_token`; cache identity excludes only the ephemeral session.
Caches cannot cross recording/timeline revisions. Cached evidence is rebound to
the current session only after identity and decoded geometry checks.

The authorized one-time reset runs before restoration, deleting only recognized
pre-UUID catalogue/practice/last-song records and known derived analysis files.
Completion is marked after cleanup; interruption retries idempotently. Audio,
prototype outputs, preferences and model/runtime caches remain untouched. Malformed,
future, UUID and unrelated files are preserved. An invalid/future completion marker
blocks startup rather than authorizing deletion. The built-in demo is transient.

Version-1 UUID envelopes are read through a one-way compatibility adapter that
keeps only the selected recording and timeline, including its authored map and
notes/settings. Reading does not rewrite the file; the next successful save writes
version 2 without history arrays. Invalid/future records remain preserved.
Replacing a reference removes old metadata links, never the external audio file.

The song envelope keeps publication to one atomic file in this slice. Catalogue
and last-song projections are separate files; projection failures are visible
warnings and preserve their previous bytes while the committed song/reference
activates coherently. An import can leave a valid unlisted song envelope; journaling and reconciliation remain deliberately out of scope for issue #3.
No multi-file crash atomicity or platform durability guarantee is asserted here.

## Verification

From the repository root:

```sh
CARGO_HOME="$PWD/.cargo-home" cargo test --locked --all-targets --features beat-analysis
CARGO_HOME="$PWD/.cargo-home" cargo clippy --locked --workspace --all-targets --features beat-analysis -- -D warnings
CARGO_HOME="$PWD/.cargo-home" cargo fmt --all -- --check
npm test
npm run check
npm run build
```

Generated fixtures exercise reset scope/interruption/symlinks, typed-ID validation,
independent copies, compatible and trimmed replacements, current-map preservation/history removal,
source geometry, stale same-song asset/session commands and UUID restoration.
Verification in this implementation pass is automated on macOS; hardware playback,
subjective listening, Windows and Linux have not been exercised for this change.
