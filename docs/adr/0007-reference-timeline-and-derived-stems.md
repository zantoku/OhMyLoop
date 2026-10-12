---
status: proposed
---

# Keep maps on the reference timeline and attach stems as derived audio

Proposed: 2026-10-12. A song selects a reference recording and a reference timeline.
Measure maps, sections and loop bounds belong to that timeline; timing analysis
always reads the reference recording, independently of the audible mix. A stem
set records the exact input asset, preprocessing and a mapping from its sample
coordinates to the reference timeline. This extends
[ADR 0001](0001-source-recording-coordinates.md) and preserves
[ADR 0004](0004-analysis-evidence-and-authored-maps.md).

An initial timeline fixes the imported reference's decoded rate, frame count and
origin. Resampling for a model or device does not redefine it. The first integrated
release produces full-song, zero-offset stem sets; validation accounts for model
padding/delay and records any approved correction rather than guessing alignment
from equal durations. Clips may be supported later through explicit span mappings.

## Consequences

- Original playback and every compatible stem set use one transport, map,
  metronome and count-in schedule. Selecting stems cannot mutate accepted timing
  or redirect beat analysis to a stem or reconstructed sum.
- A volume-adjusted replacement retains the song UUID. Reusing its timeline
  requires explicit confirmation of unchanged timing and compatible geometry;
  matching length alone does not prove correspondence. Different decoded rates
  need an explicit origin/rate mapping, with canonical timeline coordinates kept.
- A trimmed, shifted or different recording requires a new timeline. Keep the
  previous sections/map/loops with the old timeline, and offer deliberate transfer
  later; do not silently clamp, stretch or discard them. Notes remain song-level.
- Even a confirmed timing-compatible replacement invalidates automatic evidence
  for the new reference input. Existing stems retain their old input provenance
  and can be auditioned if timeline-compatible, clearly marked as coming from a
  previous reference recording. Regeneration creates a new set.
- Mix/settings are song-level; positional annotations are timeline-scoped.
  Stale analysis, separation and UI replies check recording/timeline revision as
  well as song identity. Finishing a job never selects its output automatically
  for a different active song or reference revision.

Putting a separate map on each stem was rejected: instrument estimates should
not fork a song's authored musical structure. Using the currently audible mix as
the analysis source would make results depend on mute controls and model artifacts.
