# Audio prototype findings — 2026-10-08

The Rust/Rubber Band R3 direction is viable on the tested macOS machine.
The core DSP checks pass, and the user ran native playback on a USB audio
device without reported underruns or device errors. The user also reports
excellent sound quality and endorses this direction. This is enough to begin
UI exploration while continuing the open checks below; it does not establish
production readiness on all platforms.

The measurements below are the original prototype baseline. Later desktop and
measure-map evidence is tracked in [IMPLEMENTATION_STATUS.md](IMPLEMENTATION_STATUS.md).
The risk table distinguishes current implementation from remaining validation.

## Evidence

Local toolchain: Rust 1.99.0, macOS arm64, release builds. Rubber Band 4.0.0
is pinned at `e4296ac80b1170018a110bc326fd0d45a0eb27d6` and built from
verified, unchanged upstream source. Formatting and Clippy pass.

Eleven automated tests cover:

- Actual R3 audio output at independent speed/pitch settings, including 0.25x,
  2x, and octave shifts. Duration tolerance is 50 ms; pitch tolerance 10 cents.
- Seeking and looping into a different-frequency section, checked by analyzing
  the produced audio rather than only the position metadata.
- Live control changes reaching the DSP without restarting it.
- Repeated half-speed loop pulse spacing, within 10 ms in the synthetic test.
- Actual loop seam discontinuity reduced by at least 90% with paired fades.
- Zero heap allocations in the callback consumer, including seek/pause/starvation.
- Paused audio preservation and rejection of stale seek generations.
- Threaded worker prebuffering and seek/loop command processing.
- Session marker persistence and invalid-control rejection.
- WAV decode fidelity and finite end-of-file draining.

Representative local R3 probe results for four seconds of 440 Hz source audio:

| Speed | Pitch | Output duration | Measured steady pitch | Compute time |
|---|---|---|---|---|
| 1.0 | 0 st | 4.000 s | 440.000 Hz | 0.071 s |
| 0.5 | 0 st | 8.000 s | 440.000 Hz | 0.123 s |
| 1.0 | +7 st | 4.000 s | 659.255 Hz | 0.103 s |
| 0.5 | -5 st | 8.000 s | 329.628 Hz | 0.120 s |
| 0.25 | -12 st | 16.000 s | 220.000 Hz | 0.139 s |
| 2.0 | +12 st | 2.000 s | 880.000 Hz | 0.057 s |

These are whole-render timings, not callback deadlines or worst-case CPU
guarantees. All R3 tone measurements are far inside the test tolerance.
The measurement JSON and generated listening files live in `artifacts/probe`;
they can be regenerated with `ohmyloop probe`.

The optional codec check also passed for generated PCM, FLAC, MP3, AAC-LC,
ALAC, and Vorbis files at 44.1 kHz. Each file was decoded by the app and
rendered with 0.75x speed and +3 semitones into a two-second stereo WAV.
This establishes basic codec integration, not coverage of every file variant.

R2 comparison probes at slow speeds fail the same zero-crossing pitch check,
with approximately 11-cent deviations. A separate spectrum inspection shows
multiple nearby peaks rather than a simple uniform detuning. This estimator
is sensitive to modulation. R3 is selected for playback; R2 is retained only
for comparison measurements. No claim is made about general R2 quality based
on these sine-wave checks alone.

## User-reported hardware smoke test

At 15:54:51 Europe/Berlin, the user ran `ohmyloop smoke --demo` outside the
sandbox, on `KT_USB_AUDIO` at 48 kHz:

| Metric | Reported value |
|---|---|
| Device callbacks | 938 |
| Starvation/underrun events | 0 |
| Device errors | 0 |
| Maximum worker block render time | 1.59 ms |
| Last seek command/setup time | 1.33 ms |
| Queued audio at final status | 44.7 ms |
| Source reader loop wraps | 3 |

This exercises live controls, seek, and looping through the actual native
audio callback. It does not measure when sound reached the speaker, nor does
the PASS assess timbre, clicks, or practice comfort. The exact submitted log
is recorded in `docs/DEVICE_SMOKE.txt`. Sandboxed prototype runs exposed no CoreAudio devices. Later desktop/device
checks ran outside that restriction; see the implementation status for evidence.

The user's subsequent listening assessment was: "Sound quality is excellent.
This is the way forward." This is positive subjective validation for the
material and settings tried; the conversation does not specify a broader
listening corpus or establish quality on every recording or setting.

## Decisions and open checks

| Risk | Current decision / evidence | Remaining check |
|---|---|---|
| Independent tempo and pitch | R3 passes synthetic checks; user reports excellent sound quality | Broader listening corpus, especially at extremes |
| Processing headroom | R3 is well below real time locally; USB smoke reports 1.59 ms max block time | Longer stress runs, low-end CPUs, other devices |
| Loop transitions | Source wraps keep DSP state; paired fades reduce the tested seam and preserve nominal period | Musical feel, short loops, pitch-shifted loop seams |
| Seek responsiveness | Generations invalidate queued old audio; command acknowledgements establish applied state | Physical latency and perceived seek gap; already submitted hardware buffers cannot be recalled |
| Playhead and marker accuracy | Bounds stored in source seconds, then quantized to frames; output position explicitly estimated | Precise mapping through adaptive stretching and live rate changes, plus hardware latency |
| Sample-rate mismatch | Prefer source-rate output; delay-compensated band-limited Rubato fallback and generated conversion tests now implemented | Broader device rates and listening validation |
| File decoding | Six generated codec fixtures decode and render; explicit mono/stereo policy | Broader codec/metadata corpus, unusual AAC variants, long recordings |
| Portability | Platform-specific audio backends and upstream single-file native build | Linux/Windows CI runs and physical-device smoke tests |
| Distribution | Cargo dependencies locked; Rubber Band source and beat-model provenance/notices bundled; local macOS app builds | Linux/Windows builds, installers, signing, full release dependency notices |
| Recovery | Device/worker errors reported | Device removal, reconnection, output device selection |

For a listening pass, follow the step-by-step comparison in the README.
Run `probe` and tests before changing DSP settings. Keep CI and hardware
results distinct: a configured job is not evidence of a successful run.
