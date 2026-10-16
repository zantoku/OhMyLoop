# Integrated stem separation

Open a managed-library song, choose **Stems**, and explicitly **Download model**.
The download is shared across songs and leaves playback available. Once the model
shows **Ready**, choose **Separate song**. Download completion never starts
separation automatically. Replacing an existing set requires confirmation.

Separation pauses at the current source position. Playback, seeking, speed, pitch,
loop/count-in and mixer edits stay disabled until processing ends. Close Stems
with its leading X to return to Practice; the ongoing-work indicator reopens it.
Notes and manual measure-map editing remain available. Switching songs/libraries,
recording replacement, deletion and manual stem mutation wait until processing ends.
Timing analysis and separation share one heavy-work slot, with no queue.

The large separation progress bar uses fixed stage weights calibrated from two
full-song development runs. Measured bytes, frames and inference chunks drive
progress within their stages. Unknown work holds the overall bar while its step
remains indeterminate. Manual attachment and playback-cache preparation retain
task-unit progress. The checklist follows active work; scrolling away pauses
following, and returning to the active step resumes it. Elapsed time is shown
without an ETA. **Cancel** becomes **Stopping** until child and native
cleanup finish; **Finishing save** cannot be cancelled. Normal quit asks **Keep app
open** or **Quit and cancel**, which waits for cleanup or a save already in progress.

Completion attaches the validated tracks automatically. Tracks whose whole-file
peak is strictly below −60 dBFS are omitted; the checklist retains their Skipped
state. If every track is omitted, the previous set stays unchanged. **Go to Practice**
returns to the existing mixer, still paused at the original position. Matching saved
gains and mutes survive replacement; transient solo clears. Neutral track controls
play Original; any gain/mute/solo adjustment uses stems with common fixed headroom.
The reference remains the source for waveforms and measure maps.

Failures preserve the previous association and provide explicit Retry, a short
reason and expandable **Processing details / Copy details**. A checksum mismatch
requires explicit **Model management → Reinstall model**. Removing the model leaves
completed stems playable; removing stems leaves the reference, notes and map intact.

## Development setup

### Quick manual test

1. Launch with `CARGO_HOME="$PWD/.cargo-home" npm run desktop`. Keep the existing
   development runtime; setup below is only needed if it is missing.
2. Open a song (M4A is supported), open **Stems**, and choose **Download model**.
   Check that progress updates and playback remains available. Download completion
   should show **Ready** without starting separation.
3. Pause at a recognizable position, then choose **Separate song**. Playback and
   mixer controls should stay disabled while the checklist progresses. Close Stems,
   edit a note, and reopen through the ongoing-work indicator.
4. On completion, choose **Go to Practice**. Position should be preserved and playback
   still paused. Play, mute/solo tracks and drag gains; check smooth sound. Reset
   controls to neutral to hear Original. Check that notes and the measure map remain.
5. Start a replacement, confirm it, then cancel during processing. Wait for
   **Stopping** to finish; the previous stem set should remain usable.
6. Optionally remove the model through **Model management**. Existing stems should
   still play, including after restarting the app. Separating again needs a download.

Allow several minutes for a full song and several GiB of RAM. In local debug QA,
Bones took about seven minutes end to end; this is a reference, not a time limit.
For old prototype weights/cache cleanup, see [the cleanup script](STEM_DATA_CLEANUP.md).

### Runtime setup

Issue #6 uses an explicitly prepared workspace runtime. It does not install Python
or packages from the app, search for arbitrary system Python, import prototype
weights, or automatically acquire anything on opening Stems.

```sh
scripts/stem_prototype/setup.sh
npm install
CARGO_HOME="$PWD/.cargo-home" npm run desktop
```

The pinned adapter also requires FFmpeg on PATH. Setup prepares Python 3.12.13 and
`scripts/stem_prototype/requirements.lock` under `.tools/stems`; the app probes the
pinned Python and key package versions and reports useful setup instructions if
missing. The selected execution profile is CPU, two intra-op threads, one inter-op
thread, float32, seed 0, autocast off and no per-stem normalization. There is no
accelerator/quality chooser. Processing can use several GiB of memory. Existing
reference-decoder safety limits remain; there is no additional inference duration
cap, RAM gate or overall inference timeout.

A release build deliberately reports that an installed inference runtime is not
packaged yet. Production packaging and Linux/macOS/Windows release proof are #7.
The development app reads `resources/separation/worker.py` from its compiled-in
workspace root. Moving a built development executable away from that workspace
is not an installed-runtime delivery method.

Model storage is `<Tauri app-data>/models/separation/roformer-sw-v1`. The usual
macOS default is `~/Library/Application Support/org.ohmyloop.desktop/models/separation/roformer-sw-v1`.
**Model management** displays the resolved authoritative path. The package is
699,416,805 bytes: `BS-Roformer-SW.ckpt`, `BS-Roformer-SW.yaml` and a small
`package.json` marker. Install verifies exact sizes/hashes before publication;
every separation verifies hashes again. Typical model/file edits are also detected
through metadata rechecks around reading. There is no file monitor or model import.

New stems are generated privately at
`<managed-library>/songs/<song UUID>/stems/<fresh set UUID>`. Their exact WAVs are
validated in place, without another six-WAV import copy. Playback PCM is prepared
on that filesystem and adopted into immutable `playback-cache` generations. Source
input, request, result and manifest scratch files are removed before publication;
verified provenance remains in the song record. Publication atomically changes
`song.json` and then removes the old set. Ordinary failure/cancellation cleans only
this attempt. There is no journal, resume, startup orphan scan or crash recovery.
See [manual data cleanup](STEM_DATA_CLEANUP.md) before removing leftovers.

The Stems card shows the current recording’s approximate processing-storage need.
A one-time disk check admits installation and separation. The conservative processing
estimate is `8*I + 8*O + R + 64 MiB` (stereo float32 prepared/current-output/source PCM),
about 1.95 GB for 300 seconds at 48 kHz. All six roles count before silence filtering.
This is neither a reservation nor a promise that storage remains available.

## Worker protocol v1

Rust directly launches the selected executable with `-I worker.py request.json`.
The bounded request declares version/job/package/profile IDs, local package/input/
output paths, full source/prepared geometry and digest, fixed settings and exactly
six canonical filenames. No shell or prototype CLI is launched. The child is offline;
upstream download fallbacks fail. It consumes full 44.1 kHz stereo float PCM only.

Stdout is structured JSONL: version, job ID, known semantic stage, optional
completed/total/unit. Loading is unknown; inference reports the unchanged pinned
MDXC loop's actual completed chunks. Total is `ceil(max(N,409600)/352800)`. Human
library output is redirected to stderr. The scoped iterator wrapper yields identical
indices/order and reports after overlap-add work, restoring the upstream iterator
in `finally`; it does not modify DSP arithmetic.

A clean child exit and bounded matching result are required before native validation.
Result contains the exact job/input/package/profile, actual runtime/CPU, six safe
role filenames/digests/full geometry and corrections. Rust rejects duplicate fields,
unknown roles, unsafe paths, bad identities/hashes/geometry/non-finite samples and
unsupported corrections. Validated corrections are retained in owned provenance
and validated again on load; compatible older records without that field remain
readable. Only the pinned short-input right-zero-pad-to-409600/crop
0..N declaration is allowed for 0<N<409600; normal inputs declare none.

Bounds: request/result 1 MiB, progress line 64 KiB, retained stderr/current-attempt
details 16 KiB. Malformed, oversized or stale progress terminates and reaps the
owned process tree. Native hash/decode/resample/cache-transfer loops check cooperative
cancellation at bounded work boundaries. Cancel versus committing shares one mutex.
No incomplete set becomes selectable. Notes/maps are reloaded at association save,
so accepted concurrent edits survive publication.

## Maintainer verification

Default tests use generated audio and fake process streams; they do not download
weights or require an inference toolkit. Focused setup/run commands:

```sh
CARGO_HOME="$PWD/.cargo-home" cargo test --locked --offline --workspace --all-targets
CARGO_HOME="$PWD/.cargo-home" cargo clippy --locked --offline --workspace --all-targets -- -D warnings
npm run check
npm test
npm run build
```

For actual package/full-song QA in isolated storage, the diagnostic example uses the
production manager/controller APIs. It explicitly downloads its own package:

```sh
CARGO_HOME="$PWD/.cargo-home" cargo run --locked --offline --features stem-separation --example separation_probe -- install /path/song.m4a /tmp/ohmyloop-separation-qa
CARGO_HOME="$PWD/.cargo-home" cargo run --locked --offline --features stem-separation --example separation_probe -- run /path/song.m4a /tmp/ohmyloop-separation-qa
CARGO_HOME="$PWD/.cargo-home" cargo run --locked --offline --features stem-separation --example separation_probe -- run /path/song.m4a /tmp/ohmyloop-separation-qa worker.infer
```

Use a debug build because release builds intentionally lack a packaged runtime.
The probe checks frozen seeking, saves a note during inference, reads persisted
notes/maps without rewriting them, and verifies cancellation retains the previous
set. It logs stage transitions rather than every UI tick. Local recordings/runtime/
weights remain local and are not portable test fixtures.

The probe also supports read-only reuse of an installed package through
`OHMYLOOP_PROBE_MODELS` (the parent of `roformer-sw-v1`), only in `run` mode.
Use a fresh scratch root for every calibration attempt. It emits exact finalized
per-stage monotonic timings on success, error or cancellation; it writes no timing
history to user app data. Full commands, calibration tables and their development
build/cache limitations are in the verification notes.

Measured evidence and its limits: [STEM_SEPARATION_VERIFICATION.md](STEM_SEPARATION_VERIFICATION.md).

## Testing follow-ups

Implemented after maintainer testing. The agreed scope and measurement evidence
are captured below and in [the verification notes](STEM_SEPARATION_VERIFICATION.md):

1. **Measure stage timings and weight overall progress proportionally.** Run a
   full-song separation and record elapsed time for each processing stage and the
   total. Record song duration, hardware and cache state with the results.
   “Separate instruments” previously received the same overall share as much
   shorter steps. The two full-song measurements now determine weights to
   assign realistic stage weights, retaining measured within-stage progress where
   available. Overall progress should remain monotonic and reach 100% only after
   successful publication; the checklist can keep its existing per-step indicators.
2. **Automatically scroll the processing checklist as steps complete.** Keep the
   active step visible as processing advances, so users can follow progress without
   manually scrolling the list.
3. **Start pie progress indicators at 12 o'clock.** Fill clockwise from the top,
   following the app-wide rule in [UI_DESIGN.md](UI_DESIGN.md#progress-indicators).

### Agreed refinement scope

- Apply calibrated time weighting to separation only. Manual attachment retains
  its existing task-unit weighting. Auto-scrolling and pie orientation apply to
  both workflows through their shared checklist presentation.
- Measure two full-song Bones separations with the model already installed.
  Separate worker startup, model loading, inference and output preparation in the
  timing breakdown, alongside the remaining native processing stages. Record
  hardware, build mode and cache conditions. Model download is excluded and keeps
  its separate byte-based progress.
- Use fixed stage weights calibrated from these measurements. Do not learn or
  persist per-user timing histories. Within stages, use measured bytes, frames or
  inference chunks where available. Unknown work remains indeterminate; do not
  invent advancing progress. No ETA is added.
- Auto-scroll to keep the active checklist step visible. Pause following when the
  user manually scrolls away; resume when they return to the active step. Opening
  the screen during processing reveals that step immediately. No follow button.

- During unknown-duration stages, hold the overall fraction and show an
  indeterminate step indicator. Advance when the stage completes.
- Timing results are maintainer-facing verification evidence. Do not add a timing
  breakdown to the app or persistent per-user history.

All refinement questions are settled. The authoritative implementation handoff is
[STEM_PROGRESS_REFINEMENTS.md](STEM_PROGRESS_REFINEMENTS.md).

Resolved implementation findings:

- Record true worker phase starts: Python startup/input reading previously preceded
  the loading event, and the first output write preceded the writing event. Timing
  only existing transitions would attribute that work to the wrong stages.
- Align checklist order with actual validation order, or track completion without
  assuming forward traversal. The separation checklist now follows the
  manifest/cache's established alphabetical traversal, avoiding prematurely
  completed or reopened rows. Completed steps and overall progress must not regress within an attempt.
- Existing sampled full-song debug logs suggest inference occupies roughly 72%
  of total time, versus one of 37 equal task units (about 2.7%). This motivates the
  change but is not accurate enough for final calibration. Record the benchmark
  build profile; do not claim its weights are calibrated for unmeasured profiles.
