# UUID song identity (issue #2)

Implemented identity slice of [the stem integration plan](STEMS.md). Managed
library root selection and copying remain issue #3. Imported audio is still an
external file link in this slice; neither reset nor replacement deletes it.

A song gets a canonical nonzero UUID. Audio assets and reference timelines have
separate typed UUIDs. Complete-file SHA-256 is an asset integrity digest. Normal
Open song reopens a known path/exact recording; **Import separate song** creates
an independent UUID even for byte-identical audio. Notes and speed/pitch settings
belong to the song. Sections, loops, accepted measure maps and last position belong
to a versioned reference timeline, in the existing decoded source coordinates.

**Replace recording** retains the song UUID and previous assets/timelines.
**Timing-compatible recording** is an explicit user assertion and additionally
requires identical decoded sample rate and frame count. Equal duration alone
cannot establish alignment. Different-rate recordings require a new timeline in
this slice. New timelines start without positional annotations; notes/settings
survive. Selecting a retained recording restores its timeline. Changed/missing
external files fail integrity checks; Library replacement can recover them without
opening the changed file or assigning a new song ID.

## Persistence and startup

The configured practice-data directory holds:

- `uuid-schema.json`: completed pre-release reset, version 1, `uuid-song-records`.
- `library.json`: catalogue version 2; stable song UUIDs and descriptive metadata.
- `<song-uuid>.json`: atomic song envelope version 1, containing song-level notes/
  controls, immutable asset revisions, retained reference pairs and individually
  versioned timeline records (version 1).
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

Because assets are external links until issue #3, retained audio is playable only
while its linked bytes still match its recorded digest. Editing a file in place
cannot recover its previous bytes; the old timeline/asset provenance is still
retained and can be recovered by explicitly linking an appropriate recording.

The song envelope keeps publication to one atomic file in this slice. Catalogue
and last-song projections are separate files; projection failures are visible
warnings and preserve their previous bytes while the committed song/reference
activates coherently. An import can leave a valid unlisted song envelope; storage
journal/reconciliation work remains issue #3.
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
independent copies, compatible and trimmed replacements, retained annotations,
source geometry, stale same-song asset/session commands and UUID restoration.
Verification in this implementation pass is automated on macOS; hardware playback,
subjective listening, Windows and Linux have not been exercised for this change.
