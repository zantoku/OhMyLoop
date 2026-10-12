# Local prototype verification

Date: 2026-10-12. Host: macOS Apple Silicon (`arm64`), Darwin 25.6.0.
Python 3.12.13, torch 2.8.0, demucs 4.0.1, audio-separator 0.41.1,
librosa 0.11.0; all 68 package pins in `requirements.lock`. Both models used
explicit CPU and two threads. These are single short plumbing runs, not a
music-quality comparison or a representative whole-song performance benchmark.

## Automated checks

- Rust `cargo test --locked --example stem_prepare`: two passed.
- Rust formatting and `cargo clippy --locked --example stem_prepare -- -D warnings`: passed.
- Python unittest discovery: eight passed, including POSIX SIGTERM child reaping,
  cancellation/no publication, hashes/identity, finite/geometry boundaries,
  declared source-span/output metadata, sidecar preservation, padding and mix math.
- Ruff 0.14.0 format/check, `bash -n setup.sh`, `git diff --check`: passed.
- Exact `bash scripts/stem_prototype/setup.sh`: succeeded with workspace-managed
  Python, 68 pinned packages; `uv pip check` confirmed dependency compatibility.
- Stdlib Python help/models: passed without inference imports.

## Real checkpoint smoke

Source: the project's original generated 12-second demo, float stereo 48kHz.
Prepared span: first 2 seconds, source frames 0..96,000, resampled to 88,200 frames
at 44,100 Hz. All six stem WAVs are float stereo, finite, equal-length and hashed.
No copyrighted fixture or remote audio service was used.

| Model | Model load + inference + output seconds | Peak child RSS | Geometry adaptation |
| --- | ---: | ---: | --- |
| HTDemucs 6s | 1.867 | 1,840,922,624 bytes (~1.71 GiB) | None |
| BS-RoFormer-SW | 17.525 | 2,708,209,664 bytes (~2.52 GiB) | Short input right-padded 88,200→409,600 frames; declared 321,400-frame padding cropped |

Peak RSS covers the child's process lifetime, including imports and model load;
it is not model weights size. Worker elapsed begins after imports/PCM loading and
includes model load, inference and writing stem WAVs. It excludes source decode,
parent preparation and initial dependency installation. SW's pinned chunking
processes two windows for this padded input. Comparing these tiny-passage numbers
as whole-song throughput would be misleading.

Completed final manifests (ignored local artifacts):

- `artifacts/stem-prototype/db3fab6906b6d568c04387cbc696051c4de7ae79b697dd0ba34276b5f537c7d1/manifest.json`
- `artifacts/stem-prototype/b492305de7f66d371d540a7682d480a31d1e5725eac96fd19a6ec00865a1394e/manifest.json`

Re-running each exact command produced a verified cache hit. Both completed
results produced all four convenience mixes: guitar-only, estimated no-guitar,
reconstructed sum and original-minus-guitar residual. The low-amplitude generated
source needed no mix attenuation (common gain 1.0). Headroom attenuation is checked
separately with generated loud PCM in the unit tests.

An actual Demucs no-guitar mix was processed by the existing Rust engine at
speed 0.5. The render remained finite 44,100 Hz stereo and grew from 88,200 to 176,400
frames, exactly twice the duration. This is offline DSP integration evidence,
not hardware playback or a subjective listening result.

## Setup findings and limits

SW's original checkpoint URL returned an error; the downloader used the pinned
Hugging Face mirror and verified the full published SHA256
`24e7d35ee9c64415673d3fd33e06a67cac2c103c5df6267ba1576459c775916e`.
The configuration release URL succeeded and its bytes are locked locally.
The initial dependency resolution selected librosa 1.0, which no longer brought
in `audioread` expected by this adapter; the final setup pins compatible
librosa 0.11.0. A short unpadded SW input exposed a negative overlap-add offset in
the pinned adapter; the implemented explicit padding/crop policy resolves that
case, with the exact correction in the manifest. Other geometry errors fail.

All controlled caches are directed into `.tools/stems`. ONNX Runtime is imported
by the adapter even though these two models use PyTorch. Its native startup
attempted a telemetry identifier/system cache outside the workspace; the sandbox
blocked those writes and emitted warnings. The worker calls
`disable_telemetry_events()` and denies Python socket connections. This is not
OS-level network containment or a guarantee about every third-party native cache;
production packaging needs a separate cache/telemetry audit.

The initial separation-harness checks did not include Linux/Windows, accelerators,
device playback, subjective listening, guitar quality, true transient alignment
or simultaneous playback/inference. Desktop UI and existing practice data were
unchanged. Licensing remains deferred for this experiment, and distribution
permission has not been established.

## Terminal A/B mixer verification

The later standalone Rust mixer was checked on the user's whole-song
`bones-demucs-6s` and `bones-roformer-sw` directories. Both banks contain six
finite stereo WAVs at 44,100 Hz, with 13,073,408 frames (296.449 seconds) each.
Manifest source/preparation identities match. This check compares metadata and
decoded geometry, without recomputing the recorded PCM hashes. Both banks use
the same fixed gain, 0.608501 (approximately -4.3 dB), for all mute combinations.

- Five generated-audio/transport/input tests passed, including every mute mask,
  shared A/B selection at the consumed position, resampled source coordinates,
  seek bounds/end restart, invalid inputs and key-event handling.
- Release build, strict Clippy and Rust formatting checks passed.
- `stem_mixer --check` loaded the actual twelve WAVs successfully without an
  audio device.
- `stem_mixer --smoke` exercised four seconds of output, A/B switching, guitar
  mute/unmute and seeking on `KT_USB_AUDIO` at 44,100 Hz: 753 callbacks,
  176,446 consumed frames, zero underrun frames and zero device errors.
- A real terminal session verified keys 1–6, shared mutes across model changes,
  Space play/pause, five-second seeks in both directions, and Esc exit with
  terminal restoration. Isolated guitar remained selected through live A/B
  switching; the displayed position continued rather than restarting. The
  terminal session also reported zero underruns.

CoreAudio access failed inside the tool sandbox; the approved hardware and
terminal checks ran outside that sandbox. No subjective model-quality judgment,
physical latency measurement, blind-test alignment, Linux/Windows pass or
simultaneous separation/playback result follows from these checks.
