---
status: accepted
---

# Own native playback on a controller thread and render away from the callback

Retrospectively documented on 2026-10-12. The prototype and desktop commits
provide implementation evidence, not an independently recorded decision date.

Keep the Rust playback engine independent of the UI. The desktop controller actor
owns the player and CPAL streams on one control thread and serializes requests.
Decoding, file access and DSP happen outside the device callback; a playback worker
renders into a bounded preallocated queue. The callback consumes frames, converts
samples, applies short fades and updates atomics without allocating or blocking.

The desktop documentation explicitly identifies avoiding unsafe Send/Sync
implementations for native audio objects as a benefit of this ownership model.
Separating DSP from the callback follows the project's audio invariants; bounding
queued audio limits stale work when controls change. Seek generations invalidate
old queued frames while the worker explicitly resets/re-primes DSP state.

The cost is command/reply coordination, queue-underrun handling and separate
rendered and consumed positions. Control callers may wait for acknowledgements;
the callback must not. Queue size alone does not bound physical command latency,
and submitted device buffers cannot be recalled. There is no recovered record
of a measured comparison with shared native-object ownership or callback-side DSP.

## Evidence

- Prototype commit `1ada9f4` (2026-10-08) records the worker/queue/callback architecture.
- Desktop commit `b5bf140` (2026-10-08) records single-thread stream ownership and
  the unsafe Send/Sync rationale in `docs/DESKTOP.md`.
- [Desktop boundaries](../DESKTOP.md#boundaries), [controller](../../src/desktop.rs)
  and [playback worker/consumer](../../src/playback.rs) retain these boundaries.
- [Audio risks](../RISKS.md) records queue, seek and timing limitations.
