# Integrated stem separation: implementation interview

Issue: [#6](https://github.com/zantoku/OhMyLoop/issues/6).
Updated: 2026-10-14. Status: confirmed by the maintainer; implementation and
independent implementer/reviewer agents are authorized.

## Settled context

- Roformer-SW is the production model selected through the maintainer's listening
  comparison. The domain and player remain independent of the model.
- One song owns at most one stem set. No A/B, retained sets or recording history.
- UUID identity, managed library, attached-set validation/cache and live mixer
  are complete. Reuse their reference-timeline/provenance contracts and whole-track
  peak filtering below −60 dBFS. Do not rebuild the mixer.
- The maintainer approved the explicit download → Ready → Separate song →
  automatic attachment → Go to Practice flow in the interactive prototype.
  Installation is app-wide; screen closure preserves work; completion leaves
  playback paused. Model deletion does not delete stems. Replacement is confirmed
  before starting and successful publication replaces the existing set.
- #6 uses the explicitly prepared workspace-managed development runtime, with
  readiness/version checks. The app acquires weights/config, not Python/packages.
  Production runtime packaging/installed-app proof remains #7.

## Accepted implementation decisions, round one

- Q1: pause playback at separation start and disable it until terminal completion,
  failure or cancellation; preserve position and leave playback paused. This is
  simpler than concurrent inference/playback. Download-only transport policy is
  clarified in Q8 rather than assumed from the separation decision.
- Q2: keep the current song/library fixed during separation. Allow Practice,
  Notes and Measure map navigation and manual note/map edits. Block reference
  replacement, song deletion and manual stem-set mutation until the job ends.
- Q3: one shared heavy-analysis/separation execution slot, no queue, no automatic
  cancellation of the incumbent. Disabled actions explain the running work.
- Q4: reuse #5's validate/prepare → atomic association → old-set cleanup policy.
  Ordinary failures preserve the previous association and attempt cleanup. No
  journaling, inference resumption or restart recovery; abrupt termination may
  leave unreferenced files. ADR 0008 is updated to reflect this scope.
- Q5: explicit developer runtime setup, no implicit dependency installation or
  arbitrary system-Python discovery. Useful missing-runtime setup instructions;
  production packaging stays #7.
- Q6: selected Roformer-SW CPU/two-thread profile and exact pinned settings. No
  accelerator/quality controls now.
- Q7: confirm normal quit during active download/separation; Quit and cancel
  stops/reaps the process and waits for Rust work to stop. Finish publication if
  it already started, then exit. Screen closure preserves work.

The UI contract is in [UI_DESIGN.md](UI_DESIGN.md). Prototype package sizes and
progress are illustrative, not a package descriptor or performance guarantee.

## Accepted implementation decisions, round two

- Q8: leave playback available during download-only installation; pause and
  disable it when separation starts. Model acquisition does not change transport.
- Q9: shared installed model assets live in app data. Generate into a private,
  unassociated directory in the song's managed storage and validate those owned
  files directly. Do not copy generated stems through the external-import path.
  Put private playback preparation on the same filesystem where possible.
- Q10: no partial HTTP download resumption. Cancel/failure removes partial files;
  retry starts that file again while preserving fully verified artifacts.
- Q11: no RAM monitoring/admission gate or arbitrary recording-duration cap in
  #6. Show a short notice about multi-gigabyte memory use. Inference process
  failure preserves existing stems and produces a useful error.
- Q12: show Stopping while cancellation is pending, disable repeated cancellation
  and new jobs, and report Cancelled only after process/Rust work and cleanup
  finish. Preserve the incomplete checklist.
- Q13: support short recordings using the selected adapter's known right-zero-pad
  and crop correction. Validate it explicitly against prepared geometry and the
  pinned config; output must match the complete reference timeline. Do not allow
  arbitrary correction declarations.

## Accepted implementation decisions, round three

- Q14: one-time free-space checks before package installation and separation.
  Use fixed package sizes and a conservative duration-based processing estimate,
  including temporary PCM and all six roles. Explain insufficient space; no
  reservation or ongoing monitoring.
- Q15: full model/config checksum verification on installation and before each
  inference job. Stems-screen status uses cheap installed-state checks. Mismatch
  stops processing and offers explicit reinstallation, never implicit acquisition.
- Q16: freeze Play/seek, speed/pitch, loop/count-in and mixer changes while
  separation is active, including Stopping and Committing. Waveform browsing,
  notes and manual measure-map editing remain available. Restore controls with
  their previous values after the job ends; matching gain/mute values survive
  replacement, transient solos clear as in #5.
- Q17: concise failure/retry messages with expandable bounded diagnostics and
  Copy details. Retain only the current attempt's diagnostics in memory. No
  persistent attempt history or retained audio for debugging.
- Q18: do not import/reuse prototype model weights or add a model-folder picker.
  The app downloads its own verified package. Provide manual cleanup instructions
  for obsolete prototype weights, caches and outputs while retaining the #6
  development runtime; see [STEM_DATA_CLEANUP.md](STEM_DATA_CLEANUP.md).
- Q19: no overall inference deadline. Display elapsed time and maintain cancellation;
  network connections/reads still have ordinary timeouts for stalled downloads.

## Design tree and current frontier

The product-decision frontier is empty. Q1–Q19 above are settled, including the
rejection of prototype-weight reuse. The maintainer confirmed the shared understanding
and instructed implementation with an implementer and review sub-agents. Use
[the agent handoff](STEM_SEPARATION_AGENT_HANDOFF.md); update #6's contract and
capture the approved prototype off main.

## Implementation contract

The handoff specifies cohesive package, job, preparation/validation and UI-session
seams, a versioned request/progress/result protocol, cancellation/publication
arbitration, the fixed profile and targeted verification. These technical details
implement the choices above; they do not add backend selection, recovery, runtime
installation or other product features.

Package status and song-job status are separate. A download is app-owned and can
continue across song/library navigation; separation fixes its song/library and
acquires the shared heavy-work permit. Missing development runtime gives explicit
setup instructions and disables separation, not model acquisition. No model or
runtime is automatically downloaded by opening a screen or starting inference.

Start validates eligibility/free space before creating work, then pauses/freezes
the current transport when the job is admitted. Source preparation binds full
recording digest, geometry and reference revision to its PCM. Capture and recheck
file metadata around reading and immediately before publication. Reject observable
external changes rather than associate results with different audio; same-size
edits preserving timestamps retain #5's manual-edit limitations. The final save
reloads the current song document to preserve note/map edits made during processing.

Only fully validated, prepared owned outputs may be associated. One serialized
transition arbitrates Cancel versus Committing; cancel accepted first prevents
association, commit accepted first finishes and reports the save outcome. Verify
all six raw outputs before near-silence exclusion. All-quiet results fail with No
audible stems found and preserve the old set. New directories are cleaned on
ordinary failure/cancellation; no startup scan, resumption or journal.

Completion restores disabled controls and keeps playback paused at the same
position. The shared Original/stem headroom, reference-only waveform/analysis,
saved gain/mute matching, transient-solo reset, drafts/viewports and model-independent
playback contracts from #5 remain authoritative. Model removal cannot delete audio.

## Scope exclusions

Demucs integration, accelerator/quality UI, arbitrary/custom models, model-folder
import, automatic package/runtime acquisition or updates, production runtime
packaging (#7), inference/download resumption, persistent job history, journal or
restart recovery, automatic orphan cleanup, background song switching during
separation, simultaneous heavy analysis, live transport during separation,
RAM monitoring/admission or a new duration cap, arbitrary alignment, multiple
sets/A-B/history, export and effects. Obsolete code/documentation removal is #8;
the manual data-cleanup guide is not an implementation of #8.

## Existing document conflicts

ADR 0008 is accepted and refined to the decisions above. The previous #6 issue body and older detailed sections of docs/STEMS.md included
recoverable/journalled publication, previous-reference results, multiple backends
and interrupted-job handling. This confirmed interview supersedes that text where
specified; #6's issue body has been updated. Broader obsolete-document removal
remains #8.

## Grounded facts for the interview

Read-only exploration found the selected adapter in
`scripts/stem_prototype/worker.py`, its 68-package lock in
`scripts/stem_prototype/requirements.lock`, and explicit workspace runtime setup
in `scripts/stem_prototype/setup.sh`. The local tested runtime is Python 3.12.13,
audio-separator 0.41.1 and torch/torchaudio 2.8.0. Setup uses workspace-managed
Python/dependencies, not an app installer. The upstream constructor currently
requires an FFmpeg executable despite the adapter bypassing its audio I/O.

The selected checkpoint is 699,412,152 bytes, SHA-256
`24e7d35ee9c64415673d3fd33e06a67cac2c103c5df6267ba1576459c775916e`;
its configuration is 4,653 bytes, SHA-256
`b558996f1e25eb48798bd6502505a5de94c4f966d6edfb1a0420f06cc40b501a`.
Runtime lock SHA-256:
`702b2b5990fde3473f8a67ea8f2ccccfc52edd875c28e3a60144ed2ba42f9995`.
The verified checkpoint mirror revision is
`3b39120409f3c2d50e9cc4c391f4169131e9d643`.

The BONES result manifest records 296.449 seconds of input, approximately
302.408 seconds total prototype execution and 5,771,182,080 bytes peak child RSS
(about 5.38 GiB). Its profile is CPU, two intra-op threads, one inter-op thread,
float32, seed 0, autocast off and no per-stem normalization. MPS/CUDA and concurrent
playback have not been verified. Inference allocates whole-song output/counter
tensors; duration-independent playback-cache memory is not an inference guarantee.
The pinned adapter's overlap setting of 8 means an eight-second step, not eight
overlapping windows; preserve selected behaviour rather than reinterpret it.

Existing analysis process supervision and checksum-download patterns are reusable,
but there is no shared heavy-job gate. The current foreground attachment wrapper
pauses playback and blocks virtually all other commands/navigation; background
separation must own a distinct lifetime and narrower guard scope. Lower-level
owned validation/cache adoption and atomic association publication remain reusable.
They currently lack cooperative cancellation checks. Existing quit hooks flush
UI state and reap the analysis child; separation/download shutdown participants
and preparation-thread completion need implementation.

For the accepted owned-generation/same-filesystem path, additional disk use includes
one prepared input plus six output WAVs and seven playback PCM files; native
decode/resample temporary files need additional allowance. Estimate all six roles
before filtering. The earlier external-import route additionally copies six WAVs
and can copy an entire prepared bank across filesystems; #6 must avoid both copies
without reducing validation. There is no existing free-space helper/dependency.

The MDXC pin has no progress callback, but its actual Roformer loop uses a module
`tqdm(range(...))` iterator. A scoped wrapper can emit structured chunk completion
after each iteration while yielding identical indices/order; restore it in finally
and verify bit-identical generated real-model output. Expected total is
`ceil(max(prepared_frames, 409600) / 352800)`. Loading remains indeterminate; chunk
100% is not whole-job completion. Do not parse terminal progress text.

The only allowed nonempty correction is one
`roformer-short-input-right-zero-pad-and-crop` object with actual input frames N,
padded frames equal to the pinned chunk, right padding `chunk-N`, crop start 0 and
crop end N, only when `0 < N < chunk`. Otherwise corrections are empty. Existing
Rust rejects all corrections and Python only checks final geometry/hashes; both
are insufficient as the new correction-validation boundary.
