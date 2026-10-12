---
status: accepted
---

# Keep automatic timing evidence separate from authored maps

Retrospectively documented on 2026-10-12. Map documentation and subprocess
implementation are present in commit `f3121bf` (2026-10-10); this is implementation
evidence, not an independently recorded decision date.

Automatic beat/downbeat analysis produces candidates, not mutations of the
accepted musical map. Users choose meter, pulse and span, review a draft and
explicitly apply it. Preserve protected ranges and accepted material outside the
target span. Keep source-aligned suggestions available across map revisions,
but require review before using stale candidates. Bind evidence to source,
model and preprocessing identity and record the analysis-start map revision.

The map documentation distinguishes uncertain model evidence from authored
musical interpretation and corrections. This avoids reanalysis silently erasing
user work or treating model scores as calibrated correctness probabilities.
The cost is separate candidate, preview and accepted state, provenance checks
and explicit review rather than immediate automatic replacement.

Run inference in a cancellable subprocess of the executable, outside the
playback worker and device callback. Source preparation also stays outside
playback processing. The implementation bounds inference threads, kills/reaps
cancelled or failed children and discards cancelled results. This supports
cancellation while preserving playback boundaries, at the cost of process
startup, temporary files and result/progress coordination. Isolation does not
eliminate CPU or memory contention; playback resource behavior needs measurement.
No explicit historical comparison explaining subprocesses over a dedicated
in-process inference thread was recovered; that specific rationale is inferred
from the cancellation mechanism, not established as original intent.

## Evidence

- [Map boundaries and analysis scope](../MEASURE_MAPPING.md#domain-and-persistence-boundaries)
  records evidence separation, provenance, correction preservation and review.
- [Deployment research](../BEAT_MAPPING_RESEARCH.md#deployment-implications-for-the-existing-app)
  motivates keeping analysis outside playback; its conclusions are research
  inferences, not a record selecting this subprocess design.
- [Analysis jobs](../../src/analysis/jobs.rs) and [worker](../../src/analysis/worker.rs)
  implement process execution, cancellation and result validation.
- [Implementation history](../IMPLEMENTATION_HISTORY.md#architecture-decisions)
  records the analysis/persistence/playback module boundaries.
