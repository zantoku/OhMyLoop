# Stem playback and shared DSP implementation plan

Implementation contract for GitHub issue #4. Scope confirmed by the maintainer
on 2026-10-13; implemented and independently reviewed against standards and spec.
Automated and hardware smoke checks passed. Maintainer listening acceptance
remains pending; see [usage and measured results](STEM_DSP_USAGE.md).
ADR 0009 remains proposed until measured results and listening support it.

## Agreed scope

- Build a reusable native playback foundation and prove mixing before one shared
  Rubber Band processor. This is not a disposable prototype.
- Provide a terminal listening harness using that engine: play/pause, seek,
  loops, speed, pitch, track gain/mute/solo and source switching.
- Audition the reference recording and two compatible stem sets for the same
  reference timeline. Select exactly one source at a time.
- Preserve transport position, speed, pitch and loops across source changes.
  Carry track controls between stem sets by instrument role. The reference
  bypasses individual stem controls while retaining their settings.
- Use generated fixtures for objective correctness tests and existing Demucs/
  Roformer outputs for listening. Equal geometry does not prove onset alignment.
- Automated checks establish correctness and measured performance. The maintainer
  decides listening acceptability using documented listening steps. Automated
  success without listening approval is reported as awaiting listening approval.
- Desktop mixer UI, saved mixer settings, stem attachment and inference jobs are
  later deliverables.
- Bounded playback applies to the new native source path and terminal harness in
  #4. Preserve existing desktop and CLI entry points; desktop adoption belongs
  to #5. Do not broaden #4 into import or waveform-loading changes.
- Stem sets may contain subsets of bass, drums, guitar, piano, vocals and other,
  with at most one track per role. Reject unknown/duplicate roles. Absent roles
  are unavailable, and their remembered settings survive switching sets.
- Per-role gain is 0–100%, default 100%; no boost. Mute is independent of gain.
- Multiple solos form a union. Mute takes precedence over solo. Without solos,
  all unmuted available tracks are eligible. Reference playback bypasses these
  controls without clearing them. A soloed role absent from a set contributes
  no audio; do not silently reinterpret the selection.
- Before playback, prepare all requested sources with progress reporting.
  Temporary file-backed PCM is permitted for bounded-memory decoding/resampling
  and seekable playback. Preparation time and temporary disk use must be reported.
- Scan the reference and both stem sets to calculate one shared attenuation
  factor, preserving relative levels and covering every allowed mute/solo/gain
  combination. No independent normalization. Reserve click headroom and test
  actual DSP overshoot separately; attenuation alone does not establish output
  safety after Rubber Band.
- Accept full-song, zero-offset stem sets with explicit reference provenance and
  coordinate mapping. Support different sample rates by resampling. Reject
  unexplained length/offset mismatches; no guessed trimming, padding or automatic
  alignment. Existing prototype onset alignment requires a listening check.
- Target command-to-changed-output software latency at most 250 ms at normal
  speed and 500 ms across supported speed/pitch controls. Short fades and
  re-priming interruptions are allowed, preserving transport position and
  avoiding clicks. Missed targets trigger architecture review, not silent target
  relaxation. Hardware observations are separate from software latency evidence.
- Preparation and playback have a 256 MiB incremental memory budget across all
  selected sources, decode/resample scratch, caches, DSP and queued output,
  independent of song duration. Measure separately from application baseline.
  Temporary disk usage may scale with duration.
- Mix/source changes preserve transport phase: paused stays paused; count-in
  retains elapsed phase and click schedule while pending music updates; looping
  retains bounds and current position. Changes do not restart count-in or jump
  to the loop start.
- Any read failure stops playback and displays an error (a visible terminal
  message in the harness). No special keep-playing selection-failure policy,
  automatic retry, fallback source or recovery workflow in this slice.
- Prepared PCM belongs to the current harness run. Use a private temporary
  directory and remove it on normal exit; no persistent cache, cache-management
  UI or crash recovery. Interrupting preparation exits the harness.
- Harness inputs are an explicit reference-file path and one or two stem-output
  directories. Support existing prototype manifests, checking reference
  provenance, roles and timing geometry before playback. No library attachment,
  interactive file browser or automatic alignment.
- Test the full existing speed/pitch range (0.25–2.0, −12 to +12 semitones),
  alignment, gain/mute/solo, seek/loop boundaries, paused changes and count-in.
  Measure latency, memory and underruns after preparation. The listening guide
  compares the existing outputs at normal, half and quarter speed, checking
  clicks, interruptions and artifacts. Failed limits are explicit and trigger
  architectural review; do not silently narrow supported controls.
- Optional harness BPM/meter arguments create a constant-tempo measure map for
  production metronome/count-in playback. Automated fixtures also exercise
  variable-tempo maps. No authored-song-map import or editing UI in the harness.
- Retain the existing final output clamp, not a new limiter. Report pre-clamp
  peaks and clipped-sample counts. Test ordinary headroom behavior and deliberate
  overshoot protection separately; a clamp must not conceal a failed test.

## Facts that constrain the remaining decisions

- Current engine/player consume whole-file decoded audio; device-rate resampling
  also allocates a complete recording. Bounded stem playback needs a new source
  seam and incremental preparation/resampling.
- Existing controls allow speed 0.25–2.0 and pitch −12 to +12 semitones.
- The supported practice DSP uses Rubber Band R3. Metronome/count-in are mixed
  after music DSP, preserving click pitch.
- The callback uses bounded output queues, generation invalidation and short
  fades. The consumed source position is an estimate, not a measurement of
  physical audible position or device latency.
- The existing terminal A/B prototype loads both six-track banks into RAM and
  bypasses Rubber Band. It is not evidence for this work item's resource or DSP
  requirements.

## Implementation contract

### Boundaries

Introduce a small native block-source seam with actual single-audio and stem
implementations. Keep source validation/preparation, role/mix state, transport
execution and terminal presentation separate. Preserve existing single-audio
engine/CLI entry points and existing regression tests. Use the same engine/DSP
for the harness; a second direct-to-device mixer would not satisfy this proof.

All file access, decoding, preparation, resampling and mixing occur outside the
device callback. The callback retains its bounded preallocated queue and performs
no allocation, blocking or file access. Do not change the controller's native
audio ownership or introduce unsafe Send/Sync workarounds.

### Input and preparation

- Accept a reference alone or a reference plus one/two sets; three-way comparison
  is required when both sets are provided. Support reference formats currently
  accepted by Symphonia and mono/stereo decoding; prototype stems are WAV.
- Parse the supported prototype manifest version and require completed outputs,
  unique supported roles and safe relative track paths within each supplied set.
  Unknown/future manifests fail clearly without modifying them.
- Check the explicit reference's digest and decoded rate/frame count against
  recorded preparation provenance. Source span must begin at zero and cover the
  entire reference. Validate actual stem files against declared hashes/geometry.
  Informational source paths are not authoritative.
- Derive coordinate conversions from declared rates and full-span mapping. Retain
  reference coordinates independently of device rate. Test 44.1/48 kHz crossings
  and fractional rounding with generated impulses; compensate resampler delay
  explicitly. Never infer onset alignment from matching frame counts.
- Prepare seekable, file-backed PCM with bounded buffers; no full-song Audio
  allocation in the new preparation or playback path. Resampled lengths follow
  the explicit mapping/rounding contract, not guessed padding.
- Scan prepared PCM, including resampling overshoot. The common bound includes
  the reference peak and each set's maximum per-channel sum of absolute track
  samples. This covers attenuation-only gains and every mute/solo subset, even
  when removing a track removes cancellation. Choose one non-amplifying factor
  for all sources, document its safety margin, and display its value in dB.
- Show preparation phase/progress honestly, plus elapsed time and temporary disk
  size. Fail preparation on decode/read/write/validation errors. Never modify
  reference files, stem files or manifests. Clean temporary output on normal
  exit/handled failure; no repair or cleanup scan after unexpected termination.

### Mix and transport

- Expose explicit source selection and role-based gain/mute/solo commands. Apply
  related mix state as serialized worker commands, not per-track player objects.
- Original is mutually exclusive with stems. It uses the same shared attenuation
  but ignores role controls. Missing soloed roles contribute no audio, rather
  than silently soloing a different role.
- Invalidate stale rendered output on audible mix changes. Re-prime from consumed
  source position when necessary, accounting for DSP padding/delay and bounded
  render-ahead. Use existing fade policy where suitable. Do not jump forward to
  the rendered position or restart loops/count-in to simplify implementation.
- Preserve metronome/count-in settings and fixed click pitch. Music and click
  headroom must work together with final overshoot protection.
- Source EOF and seeking continue to follow existing transport behavior. Read
  errors stop playback with a visible error; no fallback, retry or recovery UI.
- Show active source, role availability/gain/mute/solo, position, speed/pitch,
  shared attenuation and errors in the terminal. Preserve familiar controls where
  possible: 1–6 mute, 0 source cycle, Space pause, arrows five-second seek, Esc exit.
  Add documented controls for solo/gain, loop, speed/pitch and metronome/count-in.
  Centralize action/key/help definitions rather than scattering handlers.

## Verification and evidence

Use generated audio and temporary manifests/sets; automated tests must not
depend on BONES or downloaded model weights.

1. Source tests: impulses, tones, mono/stereo, rate conversions, file-backed seek,
   EOF, validation failures and bounded preparation. Check source immutability.
2. Mix tests: all 64 mute masks for a six-role set, gain endpoints/intermediate
   values, multi-solo union, mute precedence, absent roles, shared controls across
   sets and original bypass. Include cancellation examples and common headroom.
3. Actual Rubber Band output: independently verify pitch and duration, source
   seek/loop content and boundaries, queue invalidation, live gain/mute/solo/source
   changes, paused edits, EOF and metronome/count-in phase. Test control extremes
   and representative intermediate combinations with documented tolerances.
4. Latency: use distinguishable generated sources and mix signatures to detect
   changed consumer output, not just command acknowledgment. Report measurement
   definition, worst observed latency, fades/interruption length and test controls.
   Include repeated and rapid changes. Targets apply to the software pipeline:
   250 ms at normal speed, 500 ms across supported controls. Report physical device
   observations separately; do not claim sample-exact audible position.
5. Memory: measure baseline and peak process memory for preparation and playback
   with short and substantially longer generated songs and both six-track sets.
   The incremental peak must stay within 256 MiB without duration-driven growth.
   Include resampling, file-backed buffers, DSP and queues; OS filesystem cache
   outside the process is not an application memory allocation. Record disk use,
   preparation time, cache strategy and steady-state underruns. Intentional fades
   and silence are not evidence that missing device frames were avoided.
6. Protection: record pre-clamp peaks and clamp counts with music, clicks and
   count-in. Ordinary generated cases must not rely on clipping to pass. Explicit
   overshoot fixtures verify safe output rather than concealing clamp activity.
7. Regressions: formatting, strict Clippy, focused new tests and existing audio
   invariants/CLI tests. Run frontend checks only if shared desktop/UI code changes.
8. Listening: provide exact commands for the existing BONES reference and both
   output directories. Start conservatively, compare reference/A/B with all tracks
   and muted vocals, then solo/per-role gains. Repeat at 1×, 0.5× and 0.25×, with
   pitch shifts, seeks, loops and rapid switches. Check clicks, gaps, onset
   alignment, transient smearing and changes in quality versus the reference.

Report automated results, hardware playback and subjective listening separately,
including machine/device/sample-rate details and untested platforms. A configured
CI job does not establish platform success. Do not redistribute listening audio.

## Completion gate

Deliver code, harness usage, reproducible benchmark commands/results and the
listening checklist. Missed memory/latency/correctness limits require an explicit
report and architecture review; do not silently weaken them. The maintainer's
listening approval is required to accept ADR 0009. Until then, report the shared
DSP proposal as awaiting listening approval even if automated checks pass.

Block/cache sizes, prefetch strategy, temporary PCM representation, thread/module
layout, conservative attenuation margin and measurement tooling are engineering
choices within these contracts. Document the choices and evidence; they do not
require another product decision unless they change an agreed behavior or limit.
