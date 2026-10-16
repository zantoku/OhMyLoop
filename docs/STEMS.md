# Managed songs, stem separation and practice mixing

Integrated #6 implementation and current usage/runtime/storage contracts are in
[STEM_SEPARATION.md](STEM_SEPARATION.md). Earlier proposed backends, journalled
publication, history and recovery below are superseded by the confirmed #6 plan.


Updated: 2026-10-14. Status: UUID identity/reset/replacement, managed storage and
the bounded native DSP proof are implemented. Shared pre-mix DSP is accepted
following automated measurements and the maintainer's successful listening trial.
Desktop stem attachment/mixing is implemented (#5); integrated local inference
is implemented for the pinned development runtime (#6). The maintainer
selected **Roformer-SW** after comparing full-song outputs through the harness;
Demucs remains an evaluation backend, not the selected production model. The
player and stem-set domain remain independent of the model. Production runtime packaging and installed-app platform proof remain #7. The user has authorized this work
beyond the previous deferred-stems scope.

Planning update: the sole user has no valuable existing app data and explicitly
authorized wiping incompatible metadata on launch. Legacy identity migration,
annotation transfer and external-link compatibility are removed from scope.

Scope update 2026-10-13: #3 uses direct-copy imports and confirmed permanent
song deletion. Journaling, restart recovery, staging/verification phases,
locking, trash/restore, storage-size displays, import cancellation and an in-app
library move are excluded. The [managed-library plan](MANAGED_LIBRARY_IMPLEMENTATION_PLAN.md)
and accepted ADR 0006 supersede the earlier storage proposal. Later separation
job publication/recovery proposals are not requirements of #3 and need their own
scope review before implementation.

## Agreed desktop scope for issue #5

The [desktop implementation plan](STEM_DESKTOP_IMPLEMENTATION_PLAN.md) supersedes
older multi-set/A/B and recoverable-publication proposals below for #5. One song
owns at most one stem set, replaced only after validated copy and atomic association.
No retained sets, A/B selectors, journal or restart recovery. Reference replacement
always removes attached stems. A minimal Stems screen owns attach/replace/remove;
#6 later extends it with inference.

Practice uses the agreed compact transport-strip knobs, inferring Original for
unity/unmuted/unsoloed present tracks and Stems otherwise. Persist gains/mutes;
solo and count-in remain session state. Persistent prepared PCM/headroom caches
avoid repeated preparation; cheap size/modification-time checks invalidate them,
without stem rehashing on ordinary open. Preparation waits with visible progress;
unavailable stems fall back visibly, and runtime read errors stop playback. Retain
only current-output-rate preparation. See the plan for all accepted decisions and
[UI design](UI_DESIGN.md) for the selected mockup.

## Agreed integrated separation scope for issue #6

The 2026-10-14 [implementation interview](STEM_SEPARATION_IMPLEMENTATION_PLAN.md)
and [agent handoff](STEM_SEPARATION_AGENT_HANDOFF.md) supersede older #6 proposals
below, confirmed and authorized for implementation by the maintainer. Roformer-SW only,
explicit app-owned model download, a separately prepared development runtime,
one fixed-song job and shared heavy-work gate, paused/frozen transport during
separation and automatic single-set publication. No prototype-weight import,
journal, restart recovery, job history, resume, GPU/quality UI or live playback
during separation. Production runtime packaging is #7. Manual obsolete-data
instructions are in [STEM_DATA_CLEANUP.md](STEM_DATA_CLEANUP.md); no files were removed.

## Outcome and first release

Import a song into a user-selected managed library, open it by stable UUID, and
retain its notes, sections and measure map independently of file edits. Open a
dedicated **Stems** screen to install a chosen model and split its reference
recording. Return to Practice to mute, solo and adjust tracks, hear the original,
and compare compatible stem sets without losing position, loops, speed or pitch.
Measure mapping and automatic timing analysis continue to use the reference
recording. Generated stems are audio assets of the song, not new library songs.

The first release supports full-song separation, one active library, one heavy
inference job at a time, original-or-one-stem-set playback, and local inference.
It also supports attaching the current prototype's completed output sets after
full validation, so the mixer can be delivered before integrated inference.
Six tracks are the current comparison case, not a permanent model assumption.
Exporting mixes, arbitrary track alignment, multiple simultaneous sets, per-stem
effects/time stretch, partial separation, recording and cloud sync are later work.

Decisions and language:

- [Song UUIDs and audio assets](adr/0005-stable-song-identity-and-audio-assets.md)
- [Managed library ownership](adr/0006-user-selected-managed-library.md)
- [Reference timelines and stems](adr/0007-reference-timeline-and-derived-stems.md)
- [Separation process and artifact publication](adr/0008-isolated-separation-and-validated-artifacts.md)
- [Shared practice DSP](adr/0009-mix-stems-before-shared-practice-dsp.md)
- [Glossary](../GLOSSARY.md)

## Current code and changes required

`src/project.rs` now stores version-2 UUID song envelopes with one asset and an
embedded reference timeline; hashes describe audio contents. `src/library.rs`
stores version-2 entries with external canonical paths. `src/desktop.rs`
verifies and decodes recordings before activation, restores by song UUID and
guards replies with asset/timeline/revision/session references. Managed storage
must preserve these contracts while replacing external playback dependencies.

`src/analysis/worker.rs` version-1 candidates include `song_key`, geometry, model
and preprocessing. Analysis input currently comes from the desktop's loaded
`Audio`. Keep a distinct reference-audio access operation when audible input can
be stems; bind new evidence to recording revision/digest and timeline ID.

`src/engine.rs` owns an `Arc<Audio>` and reads whole-song PCM by indexed frames.
`src/playback.rs` handles bounded queued output, epochs, Rubber Band, metronome
and count-in on native workers. The terminal mixer proves normal-speed A/B
operation but preloads both banks, has no Rubber Band practice integration and
is not the production player. Retain it as a comparison tool.

`ui/App.svelte` coordinates practice, library, mapping and notes. Introduce
cohesive storage, stems and mix sessions; avoid growing its event handlers with
download/process/filesystem logic. Extend central action identities, bindings
and descriptions in `ui/actions.ts`/native keymap support. All song-sensitive
commands need expected IDs/revisions, not a bare path or song UUID alone.

## Ownership and persistence

Use separate typed IDs for Song, AudioAsset, Timeline, StemSet, Library and Job.
Use UUIDs for durable domain IDs; model/preprocessing identity remains immutable
versioned descriptors and digests. Human filenames and stem labels are display
data. Asset roles are explicit: reference recording or stem.

A song record owns metadata, notes, selected reference asset/timeline, asset/set
references and persisted practice preferences. The song’s single timeline owns its positional
annotations: sections, accepted map, saved loop selection and last position.
Compatible replacement preserves these; changed timing explicitly clears them.
There is no retained-recording or map history. Temporary transport state,
unapplied map drafts, solo audition and job progress remain session state.

Asset records contain immutable relative location, content digest, decoded
geometry, role and import/derivation provenance. Stem-set records contain input
asset/digest, timeline mapping, model/config/runtime/preprocessing identities,
settings, output assets/hashes/geometry and recorded corrections. A successful
generation never overwrites an older set. Schema validation bounds counts,
string lengths, sizes and numbers, and rejects paths escaping the library.

Suggested initial layout (versioned JSON, consistent with existing persistence):

```text
<selected-library>/
  library.json                 # authoritative library UUID/schema
  catalogue.json               # rebuildable list projection
  songs/<song-uuid>/
    song.json                  # authoritative record with embedded current timeline
    assets/<asset-uuid>/<safe-import-name>
    stems/<set-uuid>/manifest.json
    stems/<set-uuid>/<role>.wav
    analysis/<input-and-model-cache-key>/...
```

Stem asset references point into their set directory; do not duplicate their
audio under `assets/`. Exact paths and schema numbers are fixed with the first
persistence slice, including round-trip fixtures. Keep original file encoding
on import. Initially keep unnormalized stereo float WAV stems, matching tested
prototype output; show real size costs before jobs. On-disk streaming PCM caches
are rebuildable and never masquerade as authoritative assets.

App data holds active-library location, last song UUID/library UUID, device/theme/
keymap preferences, runtime/model caches and a schema/reset completion marker.
Library roots contain everything needed for song-data backup; model caches are
reinstallable. A copied library keeps IDs. Independent copies are not supported
as concurrently synchronized branches. Selecting another root first flushes
notes and practice data, waits for root-owned imports to finish, pauses/unloads
the current song and validates the new manifest. A failed switch keeps the old
library selected. No writer lock is implemented; assume one accessing instance.

For #3, copy imports directly into final asset locations and atomically replace
individual JSON documents. Report ordinary errors and attempt immediate failed
import cleanup. There is no journal, multi-file atomicity or restart recovery.
Future stem validation/publication protocols are separate proposed work.

## Import, duplicate and replacement behavior

The first import asks the user to select/create a library; the demo works without
setup. Import song copies directly to the intended asset location with progress,
without post-selection cancellation or a separate verification phase. Decode for
playback; copy/decode failures are reported, excluded from the catalogue and
cleaned up immediately where possible. No restart cleanup is promised.

An exact existing digest offers **Open existing song** or **Import separate song**.
Independent imports get distinct UUIDs and physical recording copies; hashes
suggest matches, never automatic merges. Metadata edits only affect song data.
Hash-check managed audio on song open; changed content requires explicit
replacement before playback. No background watcher.

Song deletion is permanent after confirmation and appears only in the Library
edit panel. It stops/unloads an active song and leaves the Library screen open.
External originals remain untouched; no Trash/Restore. Move/backup the entire
library with the app closed and reopen it at its new location.

**Replace reference recording** is explicit and imports a new asset. A gain-only
edit keeps the song UUID and can reuse its timeline after compatible geometry
checks and the user's **Timing is unchanged** confirmation. The previous reference
link is removed; external audio is not deleted. For differing sample rates,
implement and validate rate/origin mapping
before enabling that confirmation; otherwise require a new timeline in the first
slice. Same length alone cannot establish unchanged timing.

A trimmed or different performance gets a new timeline under the same song.
The replacement dialog explicitly warns that positional annotations will be
cleared; no previous map is saved for restoration. Notes and metadata always survive.
Timing evidence is invalid for a changed input even when the accepted map remains
usable. Previously generated stems are labelled **From previous recording** and
playable only when their timeline mapping is compatible.

## Launch reset instead of legacy migration

The UUID release starts with an empty catalogue. On launch, before last-song
restoration or loading any practice/analysis state, detect the known pre-UUID
storage layout and remove its incompatible catalogue, content-keyed practice
documents (including notes, maps and sections), last-song state and derived
analysis caches. No preservation, migration ledger or legacy external-asset
adapter is required for these records. Show a brief reset notice after success;
the user has already authorized this reset, so no confirmation flow is needed.

Scope deletion to identified app-owned metadata/cache files in the configured
legacy data location, including an explicit app-data path override. Do not
recursively delete an arbitrary selected directory. Do not delete external or
managed audio, prototype manifests/stems, models, runtimes, or compatible device,
theme and keymap preferences. Resetting the catalogue does not authorize deleting
the files it used to reference. Select/create the managed root and import songs
afresh afterward. The demo remains transient.

Make reset idempotent and keyed to the specific legacy schema: finish cleanup
before writing the new schema marker or allowing new UUID data to be saved. An
interrupted cleanup retries on launch; a permission failure reports an error
instead of mixing old/new records. Subsequent launches retain UUID songs and
practice data. Malformed or unsupported future UUID-library files continue to be
reported and preserved; the pre-release waiver does not turn corruption or every
future schema change into automatic deletion.

## Deep modules and their interfaces

Design callers around a few complete operations, hiding staging, paths and
subprocess details. Suggested interfaces express intent, not final Rust syntax:

| Module | Interface and responsibility |
| --- | --- |
| Managed library | Open root; import/relink/replace recording; read/update song; attach/remove set. Returns committed records or explicit recoverable errors; owns IDs, storage validation and the scoped legacy reset. |
| Model packages | List descriptors/status; install/cancel/remove package. Owns checksum/config/runtime verification and caches. |
| Separation jobs | Start from a committed reference revision; observe/cancel job. Owns preparation, inference process, validation and publication through managed library. |
| Practice source | Read bounded aligned PCM blocks and seek/prefetch. Original and stem adapters hide resampling, file caches and track gains. |
| Practice player | Select compatible source/set, set mix, and existing transport controls. Owns DSP/history/epochs/latency reporting and musical scheduling. |

The managed-library module owns asset validity; the player receives a validated
source description and does not parse song manifests. Model adapters implement
the same process protocol for the two real prototype backends. Avoid a universal
job framework or repository abstraction for hypothetical future implementations.
Share existing progress DTOs and checksum utilities where they fit; retain
analysis and separation-specific lifecycle rules in cohesive modules.

The separation lifecycle is `preparing → inferring → validating → committing →
completed`, with `failed`, `cancelled` and `interrupted` terminal outcomes.
Cancellation accepted before committing prevents publication and reaps the child.
The committing transition is serialized with cancel: once it starts, report
**Finishing save** and finish/recover the transaction rather than reporting a
successful cancellation while leaving a completed set. Download/install has a
separate lifecycle from a song job. Retrying creates a new job ID while retaining
the failed attempt's diagnostic provenance and any already verified model files.

At commit, recheck input asset integrity and recording/timeline revisions. If the
reference changed while inference ran, retain validated output only against its
actual input provenance, marked as a previous-recording set; never attach it to the
new reference by UUID alone. Removed song/input assets reject commit. Active jobs
hold explicit references so removal waits for cancel/completion rather than
deleting input beneath a child process.

## Stems screen and practice mixer

The song header opens **Stems** alongside Notes and Measure map. The screen shows
the selected reference recording, existing sets with model/date/storage status,
and controls to attach prototype results or generate a new set. Model setup
shows available models, estimated download/disk requirements, installed/verified
status and explicit Download/Cancel/Retry/Remove. No background download starts
merely because the screen opens.

Separation shows reference input, chosen installed model and device, estimated
storage, Start/Cancel, current stage, measured progress where available, elapsed
time and a useful failure/retry message. Unknown work is indeterminate; do not
show fabricated smooth percentages or unmeasured ETA. Completion adds a set with
**Use in Practice**; it does not replace the reference or map. Leaving the screen
does not cancel a job; the header shows ongoing work. Quit cancels/reaps before
library close. App restart marks interrupted work and offers explicit retry.

Practice gets a compact mixer with **Original** and selected stem set, per-role
gain/mute/solo, Reset mix, and optional A/B selectors for two compatible sets.
Original and stems are exclusive music sources. A/B preserves shared controls
for matching semantic roles; absent roles remain unavailable and do not get
guessed from labels. Set-local roles retain separate controls. Clear solo before
switching incompatible role layouts or request explicit user choice.

Initially persist selected set and per-role gains/mutes, while solo is transient
audition state. Validate missing/deleted-set restoration and fall back visibly to
Original. Original waveform remains the practice/mapping navigation reference;
mix changes do not trigger waveform reanalysis or reset viewport/drafts. Mapping
may audition the current mix, but its analysis and displayed waveform always
refer to the reference recording, clearly labelled.

Use central actions for role toggles, source selection, screen navigation and
job controls, with scope-aware shortcuts that do not fire in text inputs. The
terminal's 1–6/0 bindings are a useful audition preset; check conflicts with
existing desktop bindings before selecting defaults. Keep keyboard operation,
accessible labels, narrow-window layout and loading/error states in acceptance.

## Playback and resource proof

Implement original/stems block sources behind the engine input seam and keep the
existing `Engine::new(Arc<Audio>, ...)` path as a single-audio adapter. One worker
mixes aligned input frames, applies common headroom and feeds one Rubber Band
instance. Device resampling, source-coordinate conversion, loop fades and DSP
padding/delay remain explicit. Mix/source changes must never restart at the
render-ahead cursor; preserve the consumed reference position and re-prime/fade.

The whole-song prototype used about 1.17 GiB just for the two BONES PCM banks.
Production must use bounded file-backed preparation/cache/prefetch, including
backward seeks and short loops. No full-song remix for a gain toggle. Disk reads
and decoding occur outside the callback; source workers must avoid starving the
render worker. Bound queues/cache use and report underruns rather than concealing
resource failures. Preflight scratch, runtime and outputs separately; actual
model-specific peak RAM must be measured, with actionable rejection/cancellation.

Apply a fixed shared headroom factor over the active compatible comparison group
and supported gain range, preserving relative model/stem levels. Show attenuation;
it may change when a new comparison group is loaded, never per mute toggle.
Reserve click headroom and protect against post-DSP overshoot without independent
stem normalization. A/B is a comparison aid, not a claim of matched perceived
loudness or exact reconstruction. Mute/switch fades and latency must be measured
with Rubber Band at slow speed. The bounded native proof and maintainer listening
trial have now accepted ADR 0009; see [measurements and usage](STEM_DSP_USAGE.md).

## Delivery slices and dependencies

Each slice is reviewable and keeps the existing player/CLI working. Build and
focused checks pass before moving on; checkboxes are acceptance goals, not claims
of completed implementation.

GitHub delivery tracking: [integration issue #1](https://github.com/zantoku/OhMyLoop/issues/1).
Native sub-issues and blocking relationships follow the dependencies below.

| Slice | GitHub issue | Blocked by |
| --- | --- | --- |
| UUID identity and launch reset | [#2](https://github.com/zantoku/OhMyLoop/issues/2) | None |
| Managed library and imports | [#3](https://github.com/zantoku/OhMyLoop/issues/3) | #2 |
| Stem source and DSP proof | [#4](https://github.com/zantoku/OhMyLoop/issues/4) | #2 |
| Stem attachment and Practice mixer | [#5](https://github.com/zantoku/OhMyLoop/issues/5) | #3, #4 |
| Model setup and separation screen | [#6](https://github.com/zantoku/OhMyLoop/issues/6) | #3, #5 |
| Packaging and release verification | [#7](https://github.com/zantoku/OhMyLoop/issues/7) | #2–#6 |

### 1. UUID domain and launch reset

Implemented storage/contracts and verification are documented in [UUID identity](UUID_IDENTITY.md).

- [x] Introduce typed song/asset/timeline IDs, versioned song/timeline records and
  explicit recording-change handling; update native DTOs, UI guards and last-song
  restoration together. Preserve old source-coordinate units.
- [x] Reset incompatible legacy metadata before restoration, with schema gating,
  idempotent interruption recovery and no legacy identity migration.
- [x] Generated fixtures prove reset removes only its intended metadata, preserves
  audio/prototype outputs/preferences, and leaves new UUID data intact on later
  launches. Check same-song new-asset stale replies and intentional duplicates.

### 2. Managed storage and imports (depends on 1)

The next implementation package is detailed in
[Managed library implementation plan](MANAGED_LIBRARY_IMPLEMENTATION_PLAN.md).
It keeps the timeline embedded in the authoritative song record; the layout
above is illustrative, not a requirement for a separate timeline file.

- [ ] First-import setup, Library settings, validated relative links and atomic
  document saves. Existing external recordings are imported afresh.
- [ ] Direct-copy imports with progress, duplicate choices and immediate failure
  cleanup; no import cancellation or separate verification phase.
- [ ] Missing/changed recording status and explicit replacement with compatible-map
  preservation or annotation reset for changed timing.
- [ ] Safe confirmed permanent deletion from the edit panel only; manual move/open
  and backup documentation. Never delete external originals.
- [ ] Focused error/path/schema/native/frontend tests; report actual platform evidence.

### 3. Source seam and DSP mixer proof (implemented and accepted)

- [x] Add bounded single-audio/stem source implementations, common headroom and
  role-based mix commands; preserve existing CLI/single-song behavior.
- [x] Test impulses/tones for alignment, resampling, source-coordinate conversions,
  gain math, every mute combination and solo precedence.
- [x] Actual DSP tests verify pitch/duration independently, seek/loop boundaries,
  count-in/metronome timing, paused edits, EOF, live gain/A/B changes and stale
  queue invalidation. Benchmark switch latency, cache use and underruns.
- [x] Hardware smoke and maintainer listening approve shared pre-mix DSP before
  committing to the screen design. The exact listening control matrix was not
  recorded; see the evidence in the harness guide.

### 4. Attach validated sets and practice UI (depends on 2 and 3)

- [ ] Add prototype-manifest adapter that checks input identity, all output hashes,
  finite PCM/geometry/mapping and supported preprocessing; copy, never merely
  retain links to scratch directories. Unsupported/missing provenance is rejected
  with a useful explanation, not guessed from equal WAV lengths.
- [ ] Provide role mixer, Original/set and A/B controls, persistent gains/mutes,
  transient solo, action scopes and missing-set fallback.
- [ ] Prove Practice/Measure map/Notes transitions preserve transport/drafts and
  timing analysis reads only the reference. Test stale replies after replacement.
- [ ] Listen to the user's two BONES sets through the real practice DSP. This
  slice delivers useful mixing without requiring a final model/runtime package.

### 5. Model installation and local separation (depends on 2; UI on 4)

- [ ] Freeze a versioned request/progress/result protocol; supervise bounded child
  execution, validate artifacts and journal publication. Test with deterministic
  generated-output adapters, then smoke each actual backend separately.
- [ ] Model download/status/cancel/retry/remove; checksum, interrupted install,
  disk preflight, offline use and no implicit acquisition during inference.
- [ ] Add Stems screen, phase progress, completion selection, background job badge,
  cancellation/quit/recovery and one-heavy-job scheduling with timing analysis.
- [ ] Test cancellation racing commit, changing song/reference mid-job, missing
  input, process crash, malformed output and corrupted cached sets. No failure
  publishes a selectable set or modifies accepted timing.

### 6. Packaging and release (depends on 1–5)

- [x] Choose Roformer-SW from the maintainer's listening comparison; the domain
  and player stay model-independent.
- [ ] Record final package/runtime provenance and accelerator defaults.
- [ ] Package local inference per Linux/macOS/Windows, without developer tools or
  system Python assumptions. Verify installed application with network disabled
  after explicit setup; probe actual runtime cache/network behavior.
- [ ] Measure representative whole-song memory/disk/time and inference/playback
  contention. Exercise relocation, backup/restore and removable-drive failure.
- [ ] Run formatting/lint/focused native/frontend checks and CI on all platforms;
  report actual platform, hardware and subjective listening results separately.

## Decisions still to settle with evidence

Roformer-SW is selected from the user's comparison; accelerator defaults remain
open. The bounded source path, output protection and live-change re-prime strategy
have measured evidence and maintainer listening acceptance. Production runtime
packaging awaits all-platform installation
proof. Different-rate replacement confirmation awaits coordinate-mapping tests.
Keep these explicit; avoid claiming that the short generated-audio inference
smokes or normal-speed terminal mixer settle them.

This document is the local delivery plan; its deliverables are now tracked in
the linked GitHub issues. The first slice now implements UUID domain records and the scoped authorized
legacy reset. No real user data was deleted during implementation verification. The existing prototype and research remain available for
model evaluation.
