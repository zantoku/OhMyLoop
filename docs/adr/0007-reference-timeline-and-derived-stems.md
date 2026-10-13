---
status: accepted
---

# Keep maps on the reference timeline and attach stems as derived audio

Decision updated: 2026-10-12. A song has one reference recording and one reference timeline.
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
- A trimmed, shifted or different recording replaces the timeline. Choosing
  changed timing explicitly clears the current map, sections, loop and position;
  explain this before the file picker. There is no retained-recording or map
  history. Notes and speed/pitch settings remain song-level. This revises the
  earlier proposal to keep annotations for every recording version.
- Even a confirmed timing-compatible replacement invalidates automatic evidence
  for the new reference input. Every reference replacement removes the attached stem set, even when its
  timeline remains compatible. Explain this removal before confirmation.
  Regeneration or attachment replaces the single set; no older sets are retained.
- Mix/settings are song-level; positional annotations are timeline-scoped.
  Stale analysis, separation and UI replies check recording/timeline revision as
  well as song identity. Finishing a job never selects its output automatically
  for a different active song or reference revision.

Putting a separate map on each stem was rejected: instrument estimates should
not fork a song's authored musical structure. Using the currently audible mix as
the analysis source would make results depend on mute controls and model artifacts.

## Desktop scope refinement for issue #5

On 2026-10-13 the maintainer chose at most one attached stem set per song, with
confirmed replacement rather than retained stem-set history or A/B management.
A replacement is validated, copied and saved before the prior set is removed.
This supersedes the earlier consequence that regeneration retains older sets.
The maintainer also chose to remove attached stems on every reference replacement,
even when the measure map is retained. Keeping older stems was rejected because
automatic switching between the replacement recording and older stem levels could
be surprising and would add history/compatibility management. Reference timelines
and immutable input provenance remain authoritative.
