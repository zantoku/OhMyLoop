---
status: accepted
---

# Anchor practice annotations to the source recording

Retrospectively documented on 2026-10-12. This records existing behavior;
the original decision date and a formal comparison of alternatives are not recorded.

Markers, sections, loop bounds and musical timing refer to the original decoded
recording, independently of playback speed, pitch and device sample rate. Existing
sections and loops persist source seconds, converted to frames by the engine;
measure boundaries and beat anchors persist integer source frames at the decoded
source rate. Musical positions and tempo summaries are derived from the map's
anchors and notation rather than a second independently editable tempo curve.

This keeps a selected passage and its authored musical timing attached to the
same audio when practice controls change. Output-time coordinates would require
remapping annotations after speed changes; independently editable timing curves
could disagree with the anchors. These are consequences of the chosen model,
not evidence that those alternatives were experimentally compared.

Playback must convert source coordinates to rendered/device time. Nominal source
positions attached to stretched output remain estimates; this decision does not
establish sample-accurate audible playhead timing or measured device latency.

## Evidence

- Prototype commit `1ada9f4` (2026-10-08) documents original-song marker/loop
  coordinates and approximate output positions in its README.
- [Desktop architecture](../DESKTOP.md#boundaries) records source-timeline independence.
- [Map boundaries](../MEASURE_MAPPING.md#domain-and-persistence-boundaries)
  specifies authoritative anchors and derived musical coordinates.
- [Engine](../../src/engine.rs), [song documents](../../src/project.rs) and
  [map model](../../src/timing/model.rs) implement the coordinate representations.
