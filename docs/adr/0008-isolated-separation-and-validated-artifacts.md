---
status: accepted
---

# Run separation outside playback and publish validated stem sets

Accepted and refined: 2026-10-14. Rust owns model installation, job state and library commits;
a separately supervised local inference process consumes prepared PCM and writes
unassociated outputs. The selected pinned Roformer-SW Python adapter is the first
development backend. Keep its versioned request/result interface independent of Python so
native inference can replace them if justified by measurements. No inference,
download or preparation enters the playback worker or device callback.

Model packages identify exact weights, config, runtime and preprocessing versions;
installation is explicit, size-bounded and checksum-verified. Completed stem sets
include input digests, source geometry, coordinate mapping, settings, output hashes
and measured corrections. Only validated artifacts become selectable. Publish
using #5's policy: validate and prepare new owned audio, atomically save the song
association, then remove the old set. Ordinary failures preserve the old association
and attempt immediate cleanup. No journal, inference resumption or restart recovery;
abrupt termination may leave unreferenced files. Cancellation accepted before committing
terminates/reaps the process and cannot publish a completed set. Once committing
starts, finish the save and report its outcome rather than falsely
reporting cancellation.

## Consequences

- Distinguish install, prepare, infer, validate and commit stages. Report measured
  bytes/frames/chunks where available and indeterminate progress elsewhere; a
  model's internal percentage is not the whole job's percentage.
- Run one heavy analysis/separation job at a time, without a queue or automatically
  cancelling the incumbent. Use the selected CPU profile: two intra-op threads,
  one inter-op thread and the pinned preprocessing/inference settings. GPU/quality
  selection is out of this slice. Resource-admission details remain in the scope
  interview; process isolation does not establish a resource or security sandbox.
- Pause playback at separation start, preserve position, and disable playback
  until completion/failure/cancellation. Leave it paused afterwards. Stems-screen
  closure preserves the job; Practice/Notes/Measure-map navigation and note/map
  editing remain available. Block song/library switching, reference replacement,
  song deletion and manual set mutation while separation is active.
- Models and runtimes are replaceable caches; completed stems play without them.
  Deleting a model must not remove audio, maps or notes. Removing a stem set must
  not delete the reference recording.
- Installed weights/configuration are shared app-data assets. Generate into a
  private, unassociated song-owned directory and prepare on that filesystem where
  possible. Validate those exact files before association; do not make an extra
  external-import copy. A failed/cancelled job removes its new private work only.
- Download-only installation leaves playback available. Partial downloads are
  discarded on cancel/failure; verified files survive retries. Cancellation shows
  Stopping until cleanup and reaping complete. #6 has no RAM monitor or arbitrary
  duration cap; display a short multi-gigabyte memory-use notice. Check free space
  once using conservative fixed-package/duration-based estimates, with no ongoing
  monitoring or reservation. Do not assume quiet-track filtering saves space.
- Accept only the selected adapter's precisely validated short-input right-zero-pad
  and crop correction, retaining exact reference-timeline geometry; no arbitrary
  offsets or inferred alignment.
- Verify package hashes on installation and before inference, with quick ordinary
  installed-status checks. Mismatch requires explicit reinstallation. Do not
  import prototype weights: app acquisition is separate. Keep current-attempt
  diagnostics bounded and in memory, without persistent history. Inference has
  no overall deadline; network operations retain ordinary timeouts.
- Freeze transport/seek, speed/pitch, loop/count-in and mixer edits for the active
  separation job. Preserve values and source position, then restore availability
  without resuming playback. This avoids snapshot/control races while allowing
  the accepted note/map editing workflow.
- On 2026-10-19 the maintainer replaced the 2026-10-16 preassembled-runtime
  decision with managed assembly on the user's machine. Do not host OhMyLoop
  runtime archives or require the app to fetch its setup recipe/worker from the
  OhMyLoop repository. Accept more setup complexity in exchange for repository
  independence and no project-operated runtime distribution. This explicitly
  replaces the earlier prohibition on package-manager setup in the shipped app;
  it does not authorize arbitrary dependencies or reliance on system Python.
  Bundle pinned uv, FFmpeg, the worker and setup manifests with the app; acquire
  an app-private managed Python and locked CPU dependencies from approved upstream
  providers after explicit consent. Require neither system Python nor a compiler,
  developer tools or administrator setup. A small set of missing dependency wheels
  may be bundled with the installer if needed, with compatibility verification.
  Verify exact acquired artifacts; unavailable or invalid artifacts fail with
  diagnostics rather than selecting newer versions, alternate indexes or source
  compilation. Fresh setup still depends on upstream availability; repository
  independence is not an offline-installation guarantee. See the
  [implementation plan](../STEM_RUNTIME_IMPLEMENTATION_PLAN.md).
- Keep runtime acquisition explicit and separate from inference. After successful
  setup, inference remains offline. Each app release selects an exact compatible
  runtime definition; reuse compatible installations without an independent
  update channel or version chooser. Verify the installed app and managed runtime
  without workspace/system-Python dependencies, beginning with macOS arm64, then
  Windows x64 and Linux x64. Intel Mac and Windows/Linux ARM inference remain
  deferred. Certificate provisioning, public app release publication and app
  auto-updates remain outside #7. Completion requires actual installed-app setup,
  native readiness, separation, offline reuse and cancellation on all three
  selected targets. CI configuration alone is not passing evidence; report
  hardware playback/listening checks separately.
  Roformer-SW remains selected after the maintainer's 2026-10-13 full-song comparison.
- Present one explicit Set up stem separation action for missing tools/model
  components, with visible download/install/verification steps and playback
  available during setup. Ready does not start separation automatically. Store
  tools in shared app data without a folder picker; explicit reinstall/removal
  preserves model weights and all library data. Block management of assets in use.
  Ordinary app launch never installs dependencies. Interrupted/incomplete setup
  is not Ready and requires explicit retry, without a journal, dependency-install
  resume or automatic recovery. Stop/reap owned setup processes before cleanup.
  Keep reusable downloads after unsuccessful attempts; clear disposable download
  caches after success while preserving the managed interpreter. Replacements
  are verified before retiring old tools, with no history/rollback UI. Check
  estimated free space once, including temporary runtime overlap.
- Use quick runtime compatibility/presence checks for ordinary status and bounded
  real native/FFmpeg/CPU probes after setup and before separation. No full runtime
  inventory hash scan on launch or background watcher. Model checksum policy is
  unchanged; failed native probes require explicit repair. Measure setup bytes,
  time and disk footprint, and full-song Mac separation time/peak memory. Verify
  the existing closed-app library move/copy/reopen workflow without adding recovery
  features or a resource dashboard.
- #6 development uses an explicitly prepared workspace-managed Python runtime,
  with readiness/version checks and useful setup instructions. The app acquires
  model weights/configuration, not Python/packages; production packaging belongs
  to #7. Separate runtime readiness from model installation.
- Quit with active installation/separation requires confirmation. Quit and cancel
  stops/reaps inference and waits for Rust preparation/download work to stop;
  publication already in progress finishes before exit. Closing only the Stems
  screen does not request cancellation.

Native conversion now was rejected because the working model comparison provides
no portability or quality evidence for it. Preserve upstream notices/provenance;
model distribution terms remain unresolved in the research, with no new licensing
gate imposed on this planning or development pass.

## Issue #5 attachment scope

On 2026-10-13 the maintainer accepted a simpler publication policy for externally
attached results: copy and validate new owned outputs, atomically save the song
association, then remove replaced outputs. Report ordinary failures and attempt
immediate cleanup; no journal or restart recovery. On 2026-10-14 the maintainer
extended this policy to #6, replacing the earlier recoverable-publication proposal.
Verified prototype provenance is translated
into app-owned records; prepared.wav is validated on attachment but need not be
retained as a runtime dependency.
