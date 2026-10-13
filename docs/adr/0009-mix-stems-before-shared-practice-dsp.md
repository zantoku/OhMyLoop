---
status: accepted
---

# Mix aligned stems before a shared practice DSP pipeline

Proposed: 2026-10-12. Accepted: 2026-10-13 following the bounded native proof,
automated measurements and the maintainer's successful listening trial.
Render original audio or one selected stem set through a
single source-coordinate transport. For stems, apply per-track gain/mute/solo
on the Rust render worker, sum aligned input blocks and pass that mix through
one Rubber Band pipeline. Metronome/count-in clicks retain their current fixed
pitch and timing rules after music DSP. This preserves
[ADR 0002](0002-native-audio-ownership-and-threading.md).

Independent time stretchers per stem were considered: they allow output-side
gain changes but multiply DSP cost and introduce separate adaptive transient
timing. Pre-mixing gives the initial practice feature one musical clock and one
DSP history, with live changes subject to bounded DSP lookahead. Transport
resets and fallback sources still require re-priming.

## Consequences

- Gain/mute/solo and Original/Stem-set A/B changes are serialized transport
  commands. Supported live block sources ramp input weights while retaining DSP
  history and bounded queued audio. Seek/reset operations invalidate stale audio;
  paused/count-in and unsupported source updates retain explicit re-prime/fade
  handling. Preserve position, controls,
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

The earlier direct-playback prototype did not establish practice-DSP quality.
The subsequent bounded R3 harness passed generated-audio correctness/protection
checks and the agreed resource/latency targets. The maintainer reported that it
worked successfully during listening and used it to select Roformer-SW over
Demucs. This accepts the shared pre-mix architecture for desktop integration;
it does not establish listening results for every recording or platform.

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
error. Automated results establish correctness/performance; the maintainer's
successful listening trial supplies the subjective acceptance evidence.
See [the implementation contract](../STEM_DSP_IMPLEMENTATION_PLAN.md).

## Desktop cache scope for issue #5

The maintainer accepted persistent, rebuildable prepared PCM and headroom caches
inside each managed song directory. Key them by audio content, output rate and
preparation/resampling/headroom policy versions. Reuse bounded readers rather than
preparing every song again on open. Remove obsolete caches with replacement and
song deletion. This changes the desktop cache lifetime, not the proof harness's
per-run temporary-storage contract. The existing whole-reference Audio allocation
for desktop waveform/analysis remains in #5; only the stem path inherits the
bounded preparation/playback design.

For the one-set desktop mixer, use the reference automatically when every present
role is at unity gain, unmuted and unsoloed; any adjustment uses stems. Maintain one
fixed shared headroom factor across those transitions, including reference playback,
to avoid a mute action changing the base playback level. Show attenuation in Stems
details. Ordinary reopening trusts managed stems rather than rehashing; cheap file
size/modification-time checks invalidate prepared caches after typical manual edits,
without promising detection when both are preserved. Keep only current-rate
preparation; rebuilding on output-rate change trades occasional waiting for bounded
cache accumulation. Missing caches rebuild, unavailable stems visibly fall back to
reference, and runtime read failures stop playback.

## Live gain-drag refinement

Desktop listening exposed scratching during rotary gain drags. A generated actual-R3
sweep reproduced repeated near-silent windows from resetting DSP history/fades on
every mix command; steady playback had none and the sweep had no underflows. Preserve
DSP history for supported live block sources and ramp input weights over 5 ms.
Reference/stem transitions use the same bounded-weight ramp. Retaining queued output
trades immediate queue invalidation for measured control latency within the existing
250/500 ms targets. Desktop live updates use a latest-value 25 ms cadence, with
immediate first/final flush and separate debounced persistence. Repeated persistence
of unchanged controls does not restart DSP. Paused/count-in changes retain the
established reset semantics; seek/loop resets still handle transport history explicitly.
