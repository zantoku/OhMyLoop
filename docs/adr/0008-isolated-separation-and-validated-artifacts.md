---
status: proposed
---

# Run separation outside playback and publish validated stem sets

Proposed: 2026-10-12. Rust owns model installation, job state and library commits;
a separately supervised local inference process consumes prepared PCM and writes
staged outputs. The existing pinned Python backends are the first development
adapters. Keep their versioned request/result interface independent of Python so
native inference can replace them if justified by measurements. No inference,
download or preparation enters the playback worker or device callback.

Model packages identify exact weights, config, runtime and preprocessing versions;
installation is explicit, size-bounded and checksum-verified. Completed stem sets
include input digests, source geometry, coordinate mapping, settings, output hashes
and measured corrections. Only validated artifacts become selectable, through a
recoverable library operation. Cancellation accepted before committing
terminates/reaps the process and cannot publish a completed set. Once committing
starts, finish/recover that transaction and report completion rather than falsely
reporting cancellation.

## Consequences

- Distinguish install, prepare, infer, validate and commit stages. Report measured
  bytes/frames/chunks where available and indeterminate progress elsewhere; a
  model's internal percentage is not the whole job's percentage.
- Start with one heavy analysis/separation job at a time, bounded CPU threads and
  memory/disk preflight. Playback can continue, subject to measured contention;
  isolation does not establish a resource or security sandbox.
- Models and runtimes are replaceable caches; completed stems play without them.
  Deleting a model must not remove audio, maps or notes. Removing a stem set must
  not delete the reference recording.
- A production Python runtime must be packaged and tested per platform. Do not
  run workspace setup scripts, depend on system Python or install packages through
  a package manager in the shipped application. Packaging and final model choice
  remain delivery gates, not claims settled by the prototype.

Native conversion now was rejected because the working model comparison provides
no portability or quality evidence for it. Preserve upstream notices/provenance;
model distribution terms remain unresolved in the research, with no new licensing
gate imposed on this planning or development pass.
