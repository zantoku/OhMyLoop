# Bounded stem playback implementation

The terminal player uses the production `Player` worker and `Engine`, feeding
one Rubber Band R3 processor. Existing desktop and CLI callers retain their
whole-file `Audio` entry points through `MemorySource`. Desktop integration is
documented in [the desktop guide](STEM_DESKTOP_USAGE.md). ADR 0009 was accepted after the maintainer's successful listening
trial on 2026-10-13.

`stems::BlockSource` supplies stereo blocks at prepared output-device rate.
`SourceFactory` reopens that source when transport changes require fresh DSP.
`PreparedComparison` validates prototype provenance, decodes each input with
Symphonia, prepares private temporary little-endian float PCM, and constructs
reference and stem adapters. `MixState` holds six role controls shared across
sets. The reader sums bounded blocks; it never loads a complete bank. Temporary
ownership is shared with readers, and normal drop removes the run's files.

Reference rate/frame count remain authoritative for maps and provenance.
`SourceFactory::reference_geometry` carries that original identity separately
from prepared device PCM, and count-in validation compares original integer
rate/frame count exactly. Fractional rate conversion does not make a valid map
belong to a different recording.
Prepared frame count is `floor(reference_frames * output_rate / reference_rate)`;
conversion uses the reference directly rather than rounding through model rate.
Transport positions use seconds and nominal prepared PCM frames; they do not
measure physical audible position or Rubber Band's adaptive transient placement.

Preparation uses 1,024-frame resampling blocks, 8,192-frame PCM reading/scanning
buffers, a 256 KiB hash buffer and bounded decoder packets. Rubato SincFixedIn
uses a 128-tap Blackman-Harris filter. Generated impulse tests showed that merely
discarding `output_delay()` left an approximately 68-frame onset shift. The new
path explicitly supplies half-kernel zero pre-roll before discarding delay;
44.1/48 kHz crossings place impulses within one output frame. This fixes the new
path; it deliberately does not change the existing whole-file resampler.
Final zero input flushes the filter to the declared coordinate length and does
not infer or repair model onset alignment.

The scan includes the reference peak and each bank's per-channel absolute-sum
bound, including resampler overshoot. One non-amplifying factor bounds music
input to 0.70, reserving 0.30 for clicks and DSP overshoot. This is a pre-DSP
bound, not a guarantee of post-DSP safety. The existing output clamp remains;
the actual `Consumer` records pre-clamp peaks and clipped-sample counts so both
software tests and device playback expose clipping.

Supported live source mix changes ramp input weights over five milliseconds
without rebuilding Rubber Band or invalidating its history. Bounded queued audio
finishes normally; measured software latency remains within the agreed targets.
Paused/count-in and unsupported source changes retain the re-prime path from the
consumed nominal position, with existing callback paired fades. During a
count-in, queued clicks retain their chronological order and remain valid across
the music generation change; they are never drained and reinserted against the
callback. Generic seeks/loop resets clear this exception. Paused changes stay
paused. Read errors terminate the worker, stop playback and become visible in
the harness; there is no retry, fallback source or repair process.

The callback reads a bounded preallocated queue, applies fades/clamping and
updates scalar atomics. Files, mixing, decode and DSP remain on workers. The
public headless player uses the same worker and Consumer for independently
paced software measurements, without owning a CPAL stream. There are no unsafe
Send/Sync overrides.

See [usage, measurements and listening checks](STEM_DSP_USAGE.md). RSS sampling
is an observed process-memory peak, not a proof that a 10 ms sampler captures
every shorter spike. Windows/Linux playback remains untested in this session.
The maintainer reported successful listening and selected Roformer-SW after
comparing the existing output sets. No persistent cache, crash recovery, desktop mixer,
library attachment or separation process is introduced here.
