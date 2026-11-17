# Issue #6 implementation handoff

Status: product choices Q1–Q19 are confirmed. The maintainer authorized
implementation with an implementer and independent review sub-agents on
2026-10-14. Contract:
[STEM_SEPARATION_IMPLEMENTATION_PLAN.md](STEM_SEPARATION_IMPLEMENTATION_PLAN.md),
[UI_DESIGN.md](UI_DESIGN.md), accepted ADR 0008 and existing #5 contracts.

## Outcome and boundaries

Inside the existing Stems screen, explicitly download the single Roformer-SW
model, wait for Ready, explicitly separate the current full reference recording,
and automatically attach one validated/prepared stem set. Reuse the current mixer,
near-silence filtering and playback-cache/headroom pipeline. Keep user-authored
reference timeline, measure map, sections, notes and viewports intact. Replacement
requires confirmation and preserves the old set through preparation/validation.

Downloads are app-owned, shared across libraries, and do not pause playback or
lock song navigation. Separation is app-owned but fixes its current song/library:
pause at admission and freeze transport/seek, speed/pitch, loop/count-in and mixer
edits until the terminal outcome. Navigation among Practice/Notes/Measure map and
manual note/map edits remains available. Never tie job cancellation to disposing
the Stems component. Restore controls and leave playback paused at its position.

No Demucs integration, custom models, GPU/quality chooser, model-folder import,
prototype-weight reuse, runtime/package-manager installation, automatic acquisition
or updates, queue, simultaneous heavy analysis, RAM monitoring/new duration cap,
inference deadline, resume/journal/restart recovery, attempt history, audio retained
for debugging, orphan scanner, multiple sets/history/A-B, export or effects.
Existing reference-decoder safety limits remain; no new inference duration cap.
Production inference runtime packaging is #7; obsolete code/docs removal is #8.

## Module seams and reuse

- Keep package installation/status, runtime readiness, separation supervision,
  source/output preparation, managed publication and UI state distinct. Small
  cohesive native modules and a dedicated app-lifetime UI separation session;
  do not enlarge App.svelte into the job implementation.
- Follow `src/analysis/{jobs,models,progress}.rs` for bounded supervision, checksum
  installation and progress patterns. Do not import analysis component-disposal
  cancellation or make separation depend on the beat model/backend.
- Add a narrow shared heavy-work gate for timing analysis and separation, with
  explicit work identity and RAII ownership. No queue or universal job framework.
  Claim before starting heavy work, release after worker/process cleanup. Busy
  attempts leave the incumbent untouched and explain why Start is unavailable.
- Reuse `src/managed/stems.rs`, `src/stems/prepare/library.rs` and cache adoption.
  Extract an owned-generation validation entry point rather than recopying output
  through external attachment. Share validation/translation logic; do not duplicate
  a weaker generated-output validator. Preserve the external attachment workflow.
- Source preparation belongs in reusable Rust library code, using the prototype's
  Symphonia/Rubato policy. Preserve `examples/stem_prepare.rs` as a CLI wrapper.
  Do not run `cargo run`, workspace setup scripts or the prototype CLI from Tauri.
- The existing foreground `Desktop::attach_stems` wrapper globally locks commands.
  It is not the background separation lifetime. Introduce explicit native guards
  and the minimum begin/commit/finish controller seam, allowing status/note/map
  operations without permitting conflicting recording/set operations.
- Centralize new action identities, availability, tooltips/help and shortcuts.
  UI disablement is not the sole native validity/ownership guard.

## Runtime and package identity

Development uses the explicitly prepared `.tools/stems/venv` and managed Python,
with pinned-version/readiness checks and actionable developer setup instructions.
Do not search PATH for arbitrary Python or bootstrap dependencies from the app.
Separate model Ready from development-runtime readiness: model Download can work
without the runtime, while Separate requires both. Keep runtime resolution
replaceable for #7; a release app must not silently depend on a workspace path.
The current upstream constructor also requires FFmpeg on PATH. Preserve/document
that development prerequisite unless removal is separately verified.

Use the exact identities/hashes recorded in the implementation plan. The worker
uses Python 3.12.13, audio-separator 0.41.1, torch/torchaudio 2.8.0, the selected
699,412,152-byte checkpoint and 4,653-byte YAML. Keep CPU explicit before model
instantiation, two intra-op threads, one inter-op thread, seed 0, float32, autocast
off, no per-stem normalization and the selected eight-second inference step.
The checkpoint mirror is revision-pinned; config acquisition must use the pinned
expected hash, rather than trusting a first download. Preserve upstream provenance
and notices. Do not add a new licensing approval gate to this development slice.

Ship a versioned, validated package descriptor with package/profile IDs, exact
expected filenames, byte sizes, SHA-256, HTTPS acquisition URLs and runtime/config
provenance. Store installed weights/config under the platform app-data directory,
separate from the library and development runtime. No existing prototype import.
Ordinary model status is cheap; full hash verification is required at installation
and before inference. Avoid an accidental extra verification pass per layer, while
ensuring the child only loads the pinned validated package.

One package installation at a time. Download into private partial files with
bounded reads/total sizes and measured byte progress; verify before publication.
Cancel/failure removes partial files, preserving verified completed files. Retry
starts incomplete files again. Use ordinary network timeouts and cancellable waits;
do not let a blocking read prevent stopping indefinitely. No partial HTTP resume
or auto-retry/download. Model removal is unavailable while it is being installed
or used by a job, removes model assets only, and leaves attached stems playable.

## Disk admission and private storage

Before install, check free space for known missing package bytes plus small overhead.
Before separation, estimate additional scratch/output/preparation bytes on the
actual destination filesystem. Do not count already-occupied reference/old-set
bytes as free space or assume silent roles will be omitted. No memory admission,
disk reservation or ongoing free-space polling. Surface check/write failures.

Use checked integer arithmetic and exact reference geometry. A conservative
same-filesystem starting estimate is `8*I + 8*O + R + 64 MiB`, where I/O/R are
stereo float32 sizes for full 44.1 kHz prepared input, current-output-rate PCM and
source-rate PCM respectively. Verify that the implemented temporary layout fits
the bound; increase/document it if it does not. This estimates work units/storage,
not an exact promise. At 300 seconds/source-device 48 kHz it is about 1.95 GB
including the allowance. Package installation is separate (~699 MB).

Create a fresh checked `songs/<song UUID>/stems/<set UUID>` directory which is not
associated in song.json. Generate WAVs there and use job-private preparation on
that filesystem. Validate before adoption/association, then remove transient
prepared input/request/result/work files that are not playback dependencies.
Reuse immutable playback-cache generation ownership and bounded cross-filesystem
fallback where unavoidable. No full-bank memory load or duplicate stem copy.
Cleanup only this job's new files on ordinary failure/cancel. Existing readers and
the old association remain intact. Crash leftovers are manual, not auto-recovered.

## Versioned worker boundary

Freeze and document protocol version 1 before parallel implementation. Rust owns
job/source/package truth and final publication; the direct Python child owns only
inference and writing declared outputs. Use a request file, bounded JSON-lines
progress on stdout, bounded diagnostic stderr, and a bounded result file. Launch
the selected executable directly with argv, not shell interpolation. Keep human
library logs on stderr, so stdout contains protocol messages only.

Request fields: protocol version, job ID, package/profile ID and pinned identities,
local package paths, prepared WAV path/digest/rate/channels/frames, fixed settings,
output directory and exact canonical role filenames. No remote source recording,
network acquisition, arbitrary command or caller-chosen backend.

Progress fields: protocol version, job ID, semantic stage ID, optional completed/
total/unit. Validate IDs, known stages/units, monotonic counts and bounded message
sizes. Unknown model-load work is indeterminate. Keep phase fractions separate
from whole-job completion. Publish at the existing throttled cadence while
preserving stage transitions and terminal events. Do not parse terminal tqdm text.

Result fields: protocol version, matching job/input/package/profile identity,
actual runtime/device, exactly six known output roles with safe relative filenames,
digests and full rate/channel/frame geometry, and declared corrections. Success
requires a clean child exit and valid result, followed by native validation. A
child's complete result is not a published or selectable stem set.

Suggested bounds: 64 KiB per progress line, 1 MiB request/result, 16 KiB retained
diagnostic tail, with bounded readers even for malformed/unterminated lines. The
parser must never grow memory indefinitely or trust excess worker data. Worker
fixtures should exercise these boundaries without installing Python/model weights.

For measured chunk progress, scope an instrumented wrapper around the pinned
MDXC module's tqdm iterable: yield identical loop indices/order, report completion
after each model/overlap-add iteration resumes, restore in finally. Total is
`ceil(max(prepared_frames, 409600)/352800)`. Verify instrumented versus existing
real-model output bit for bit on generated input. Do not rewrite DSP arithmetic,
normalize outputs or reinterpret overlap=8. Model load, output writing/hashing,
native validation, preparation and publication are distinct stages.

## Reference and artifact validation

Pin song/library generation, reference revision/token, digest and source geometry
at admission. Prepare the entire reference span at 44.1 kHz stereo float using
existing delay compensation/flooring. No excerpts, offset inference or separate
per-stem timelines. Hash/capture metadata around preparation so PCM has trustworthy
input provenance. Recheck reference/session/file metadata immediately before
publication and reject observable changes. Preserve the established same-size/
preserved-mtime manual-edit limitation; don't introduce a filesystem monitor.

Validate hashes, finite decoded samples, canonical roles, safe owned paths,
provenance and complete geometry for all outputs before peak omission. Near-silence
means whole-track max absolute sample strictly below 0.001 (−60 dBFS), not RMS.
Retain a track at the boundary or with a brief audible sample. Excluded owned files
are removed and skip downstream work; checklist Skipped survives completion.
All excluded gives No audible stems found; keep the previous set.

The only permitted nonempty correction is the exact known short-input right-zero-
pad/crop declaration described in the implementation plan: pinned chunk 409,600,
actual prepared N, right padding chunk−N, crop 0..N, only for 0<N<chunk. Confirm raw
and final geometry. Reject missing/extra/contradictory corrections or arbitrary
offsets. Share this narrowly defined validation where supported prototype results
use the same declaration; preserve rejection of unsupported provenance.

Reuse validated PCM/headroom directly for persistent cache adoption. Never rerun
the old duplicate preparation flow. Input snapshots preserve metadata captured
before validation, immutable generations preserve existing readers, and failed
adoption must not replace a usable cache pointer. Model/runtime removal is not a
playback dependency or a reason to invalidate output-based PCM identity.

## Cancellation, save and quit

Make preparation, hashing, copying, decode/resampling, output validation and cache
creation cooperatively cancellable at bounded work boundaries. Killing Python
alone does not cancel native preparation. No unsafe Rust-thread termination.
Supervise a direct child, drain bounded streams, poll exit responsively and kill/
reap on cancellation, protocol failure and shutdown. Handle owned process-tree
cleanup for subprocesses; do not leave an inference descendant behind.

Serialize the Cancel/Committing transition. Cancellation accepted first prevents
association and cleans the new job. Committing accepted first disables Cancel,
shows Finishing save, and completes/reports the atomic save. Stopping remains
active until native/child cleanup completes and the heavy-work permit is released.
Do not report Cancelled early or allow another job during cleanup.

At commit recheck current library/reference/prior-set ownership; reload the latest
song document so concurrent accepted notes/maps survive. Preserve matching saved
gain/mute, default newly present roles to unity/unmuted and clear transient solo.
Do not reselect Original/Stems explicitly; use existing neutral-control inference
and fixed common headroom. Replace player input safely, preserving source position
and transport values, then remove the old set. Audio-device unavailability must
use the existing visible fallback/error contracts, not discard validated audio.
Cleanup failure after association is a completed result with a useful warning.

Use the existing prepare-exit handshake: confirmation for active installation/job,
Keep app open or Quit and cancel, flush notes/settings, stop/reap/join work, then
close. If commit already began, finish it before exiting. Screen closure only
navigates. No persistent interrupted-job record/restart workflow or automatic retry.

## UI, documentation and review evidence

Rewrite the approved flow using production Svelte modules; do not promote the
throwaway HTML/reducer. Show the selected model card, truthful installation/
runtime status, actual sizes, explicit Download/Separate, replacement confirmation,
semantic progress checklist, Cancel/Stopping/Finishing save and completion/Go to
Practice. Use the project's top-leading X and existing component/theme conventions.
Keep installed-model management/manual output attachment secondary. A compact
ongoing-work indicator opens Stems from the active song screens. Keyboard, narrow
820×620 and ordinary 1180×810 layouts must remain usable and scrollable.

Errors have a concise explanation, explicit Retry and expandable/Copy details.
Keep bounded current-attempt diagnostics in memory, no persistent history or audio.
Job/package IDs guard stale events/replies; reopening Stems reads app-owned status.
Separate package progress from per-song progress; no fake ETA or fabricated
percentage while work is unknown. Whole workflow reaches 100% only after association
publication. A successful reply preserves final skipped/completed rows despite
event/reply ordering, as attachment does today.

Generated-data automated evidence should cover package size/hash/partial retry/
offline behavior, protocol corruption, crashes/hangs/stderr bounds, cooperative
cancel at native stages and race against commit, quit cleanup, shared heavy-work
exclusion, stale reference/library events, external file edits, short/normal inputs,
all-quiet/boundary/brief-note roles, PCM/cache reuse and failure cleanup. Verify
note/map drafts and persisted edits while the job publishes, frozen controls and
matching-role restoration, and model removal leaving playback available. Default
tests must not download weights or depend on an installed inference toolkit.

Then run the real Roformer CPU adapter on generated short and normal input, compare
instrumented output to the selected old adapter, and perform one actual desktop
full-song run with the maintainer's local reference. Record source/runtime/profile,
stage timings, peak memory, extra disk, progress and actual cancellation. Local user
audio is not a portable fixture; do not commit recordings/weights/runtime/builds.
Keep physical hardware/listening and Linux/macOS/Windows evidence distinct; #7
remains the installed-runtime/platform release gate.

Run fmt, strict workspace/all-target Clippy, focused native/frontend tests, Svelte
checks and production frontend build. Broaden once to the required full suites
when integration is complete; do not repeatedly hammer paced DSP tests under
parallel CPU load. Preserve CLI/harness and existing gain/transport regressions.
Update usage/run instructions, runtime prerequisites, exact model/app-data paths,
manual cleanup guidance and evidence. Independent Standards and Spec reviews
must compare against this frozen handoff/plan and report findings separately.
