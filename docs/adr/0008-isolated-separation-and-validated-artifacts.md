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
- A production Python runtime must be packaged and tested per platform. Do not
  run workspace setup scripts, depend on system Python or install packages through
  a package manager in the shipped application. The maintainer selected
  Roformer-SW on 2026-10-13 after comparing full-song outputs through the shared
  DSP harness. Production runtime packaging remains a delivery gate.
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
