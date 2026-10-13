---
status: proposed
---

# Mix aligned stems before a shared practice DSP pipeline

Proposed: 2026-10-12. Render original audio or one selected stem set through a
single source-coordinate transport. For stems, apply per-track gain/mute/solo
on the Rust render worker, sum aligned input blocks and pass that mix through
one Rubber Band pipeline. Metronome/count-in clicks retain their current fixed
pitch and timing rules after music DSP. This preserves
[ADR 0002](0002-native-audio-ownership-and-threading.md).

Independent time stretchers per stem were considered: they allow output-side
gain changes but multiply DSP cost and introduce separate adaptive transient
timing. Pre-mixing gives the initial practice feature one musical clock and one
DSP history, at the cost of changes being subject to DSP lookahead and re-priming.

## Consequences

- Gain/mute/solo and Original/Stem-set A/B changes are serialized transport
  commands. Invalidate stale queued audio, re-prime from the consumed source
  position when needed, and use short paired fades. Preserve position, controls,
  loop, count-in intent and shared track controls across compatible sets. Measure
  resulting latency/artifacts; queue duration does not equal physical latency.
- Use one documented common headroom policy for compatible A/B sources with
  bounded gains. Never normalize stems independently or sum original plus stems
  by default. Preserve model output levels and disclose attenuation. Include
  music DSP overshoot and click headroom in output protection tests.
- Use bounded file-backed PCM reading/preparation outside the callback rather
  than keeping every full-song comparison bank in RAM. Seek and loop prefetch
  belong to the source implementation; the callback still consumes a bounded
  preallocated output queue. The exact cache size is a measured implementation
  choice, not an ADR constant.
- Retain the existing single-audio engine entry point for CLI callers. A block
  source interface hides original-versus-stems reading and mixing; use actual
  single-audio and stem adapters, not a generic track workstation abstraction.

This is an initial architectural proposal. Prototype A/B playback at normal
speed does not establish that pre-mix updates under Rubber Band meet practice
latency and quality requirements; the plan includes a focused proof before UI work.

## Agreed proof scope

On 2026-10-13 the maintainer agreed to prove the reusable native source path in a
terminal harness, preserving existing desktop playback until integration. Prepare
the reference and up to two compatible full-song stem sets into temporary,
file-backed PCM before auditioning. A bounded scan establishes common attenuation;
role gains are attenuation-only. This trades preparation time and temporary disk
space for predictable seeking, switching and bounded RAM. Prepared files are
disposable per run, with no persistent cache or crash-recovery mechanism.

Targets are 256 MiB incremental preparation/playback memory independent of song
duration and software mix-change latency of 250 ms at normal speed, 500 ms across
supported controls. Keep the existing final clamp rather than introducing another
limiter; report clipping instead of hiding it. Read failures stop playback with an
error. Automated results establish correctness/performance; acceptance still
requires the maintainer's listening review, particularly at slow playback.
See [the implementation contract](../STEM_DSP_IMPLEMENTATION_PLAN.md).
