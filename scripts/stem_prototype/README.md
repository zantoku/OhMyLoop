# Offline guitar stem prototype

Standalone experiment for the [implementation plan](../../docs/STEM_SEPARATION_PROTOTYPE_PLAN.md).
This does not change desktop playback, saved sections, notes or musical maps.
It compares direct HTDemucs six-stem inference with the BS-RoFormer-SW model
used by feedBack, using the app's Symphonia decode and delay-compensated Rubato
resampling. The six outputs are bass, drums, guitar, piano, vocals and other.

## Setup and acquisition

Run from the repository root. Prerequisites are the existing Rust/native build
setup (see root README), `uv`, and FFmpeg on PATH (`audio-separator` expects it).
The reference environment is Python 3.12.13, torch/torchaudio 2.8.0,
audio-separator 0.41.1 and demucs 4.0.1. `requirements.lock` pins all 68 resolved
packages from the locally verified macOS Apple Silicon environment.
librosa is pinned to 0.11.0 because this adapter expects its audioread dependency.

```sh
bash scripts/stem_prototype/setup.sh
python3 scripts/stem_prototype/cli.py --help
python3 scripts/stem_prototype/cli.py models
.tools/stems/venv/bin/python scripts/stem_prototype/cli.py download --model demucs-6s
.tools/stems/venv/bin/python scripts/stem_prototype/cli.py download --model roformer-sw
```

Setup installs tools, Python, caches and its environment under `.tools/stems`.
It does not download checkpoints. Acquisition is explicit and records download
URLs, full SHA256 and byte sizes in a model lock. Demucs has an upstream checksum
prefix; SW is checked against the full published checkpoint checksum. SW's
original host is unavailable: acquisition falls back to a pinned Hugging Face
mirror revision and records the successful URL. Configuration bytes are hashed
on first acquisition, not backed by an independent publisher checksum. Preserve
`model-lock.json` when reproducing a comparison. Changed artifacts are rejected;
an unlocked preexisting artifact requires manual inspection/removal.

The full lock is a reference environment, not proof of portable installation.
On Windows `diffq` needs upstream's `diffq-fixed` alternative and setup.sh needs
a shell adaptation; Linux/Windows dependencies and inference remain untested.
Do not silently re-resolve a published comparison's runtime. To deliberately
choose a new environment, use `requirements.in`, freeze it into a separate lock,
and record its differences. Model files are large: Demucs approximately 80 MiB,
SW approximately 667 MiB; six stereo float stems use about 2 MiB/second.

## Separate a passage

```sh
.tools/stems/venv/bin/python scripts/stem_prototype/cli.py separate path/to/song.flac --model demucs-6s --start 30 --duration 20
.tools/stems/venv/bin/python scripts/stem_prototype/cli.py separate path/to/song.flac --model roformer-sw --start 30 --duration 20
.tools/stems/venv/bin/python scripts/stem_prototype/cli.py separate path/to/song.flac --model demucs-6s --whole
```

The default is 20 seconds from the beginning, CPU, two inference threads. Use
`--threads 1` through `--threads 8`. `--device mps` or `--device cuda` is an explicit experiment;
accelerators are not locally verified. Actual device is recorded in the manifest.
The parent streams upstream stage/progress information. Ctrl-C/SIGTERM terminates
and reaps inference, and discards the incomplete job; cancellation of native
kernels is process termination, not cooperative neural inference.

Preparation uses original source frame coordinates, floors seconds to frames,
clamps the end to the recording, and writes 44,100 Hz float stereo PCM. It hashes
the complete original file before/after decoding. A source-rate excerpt is
resampled to `floor(span_frames * 44,100 / source_rate)` frames. Mono is duplicated
to stereo. This initial harness supplies no extra boundary context; short
excerpts may differ materially from whole-song inference. The existing decoder
loads the whole recording and retains its 64 million-frame memory cap, even for
an excerpt; this is not streaming source preparation.

Outputs live under ignored `artifacts/stem-prototype/<job-key>/`. A completed
manifest appears only after finite PCM, names, geometry, source span, hashes
and safe filenames validate. The pinned SW adapter cannot process inputs shorter
than its configured chunk; these are explicitly right-zero-padded to that chunk
and the declared padding is cropped afterward, recorded in `corrections`.
Other output mismatches fail.
Only verified completed results are reusable. The key includes source identity,
prepared PCM and preprocessing, exact model/config locks, settings, package
versions, Python/platform, lock and implementation hashes. A repeat command
prepares the excerpt again before validating its cache hit. Publication is an
atomic directory rename; concurrent identical jobs are not coordinated, and
only one wins publication. Incomplete directories after an uncatchable SIGKILL
are never reused; they can be removed manually once no job is running.

Worker inference requires no network; missing assets cannot trigger implicit downloads.
Python socket connections are denied and ONNX Runtime telemetry is disabled,
but native third-party cache behavior is not fully contained; see verification.
The checkpoint runtimes deserialize locally acquired model files, so use this
controlled downloader, not arbitrary checkpoint files from strangers. Demucs's
older loader needs `TORCH_FORCE_NO_WEIGHTS_ONLY_LOAD=1` with torch 2.8; only the
child receives this setting. SW uses the pinned adapter's internal
`MDXCSeparator.demix` API and a local-only acquisition override. This bypasses
upstream input/per-stem normalization and output writers; an adapter upgrade
requires rechecking that internal interface and output geometry.
Demucs uses its reference shared mean/std normalization, shifts 0, overlap 0.25,
float32; SW uses config chunk length, overlap 8 and no autocast. Both use seed 0.

## Listen and mix

The command prints stem WAV paths you can open in OhMyLoop. To generate
practice tracks, replace `RESULT` below with the printed manifest path:

```sh
.tools/stems/venv/bin/python scripts/stem_prototype/cli.py mix RESULT --preset guitar-only
.tools/stems/venv/bin/python scripts/stem_prototype/cli.py mix RESULT --preset no-guitar
.tools/stems/venv/bin/python scripts/stem_prototype/cli.py mix RESULT --preset sum
.tools/stems/venv/bin/python scripts/stem_prototype/cli.py mix RESULT --preset original-minus-guitar
.tools/stems/venv/bin/python scripts/stem_prototype/cli.py mix RESULT --preset sum --gain guitar=0.25 --gain vocals=0.5 --output artifacts/custom-backing.wav
```

`no-guitar` sums estimated non-guitar stems. `original-minus-guitar` subtracts
the guitar estimate from the prepared original: it is a subtraction residual,
which can sound different from estimated accompaniment. `sum` is a reconstructed
estimate, not a promise that the original is recovered. Stem WAVs are unnormalized
lossless 32-bit float, which can exceed playback headroom. Each mix uses one common
attenuation when its peak exceeds 0.98; its `.mix.json` records that gain and peak.
There is no independent stem normalization. Gains are linear finite values in
[-16,16]; repeated gains for a stem use the last value. Presets supply defaults,
then custom gains override them. The residual preset rejects custom gains.
Existing output audio or sidecar files are preserved by refusing overwrite.

Derived WAVs open as independent songs. They do not inherit authored annotations
or support synchronized multistem transport in the app. Frame-count equality
checks geometry only: sample onset/perceptual alignment still needs transient
and listening evaluation. Slow the derived track with ordinary app controls to
assess artifacts. Generated tones cannot establish guitar usefulness.

## Verification

```sh
CARGO_HOME="$PWD/.cargo-home" cargo test --locked --example stem_prepare
CARGO_HOME="$PWD/.cargo-home" cargo clippy --locked --example stem_prepare -- -D warnings
.tools/stems/venv/bin/python -m unittest discover -s scripts/stem_prototype -p 'test_*.py' -v
```

Generated tests cover source-frame rounding/clamping, resampling geometry,
cache/hash rejection, finite/geometry validation, mix arithmetic/headroom,
failure publication and POSIX cancellation/child reaping. No model is mocked
and labeled as real inference. For a reproducible real plumbing smoke:

```sh
CARGO_HOME="$PWD/.cargo-home" cargo run --locked --release -- demo artifacts/stem-smoke-demo.wav
.tools/stems/venv/bin/python scripts/stem_prototype/cli.py separate artifacts/stem-smoke-demo.wav --model demucs-6s --duration 2
```

See [local verification evidence](VERIFICATION.md). No hardware playback,
subjective listening, guitar-quality, accelerator or Linux/Windows success is
implied by automated checks.

## Attribution and deferred decisions

Prototype code: GPL-3.0-or-later. Reused upstream packages remain under their own
licenses/notices; installation retains package metadata and upstream source.
No feedBack application code is copied. Its model choice motivated the SW
comparison. See the [research](../../docs/research/STEM_SEPARATION_RESEARCH.md)
for artifact provenance and permission findings, and upstream
[Demucs](https://github.com/facebookresearch/demucs),
[audio-separator 0.41.1](https://github.com/nomadkaraoke/python-audio-separator/tree/v0.41.1),
[SW mirror](https://huggingface.co/lainlives/audio-separator-models/tree/3b39120409f3c2d50e9cc4c391f4169131e9d643)
and [SW conversion provenance](https://huggingface.co/lumabeat/bs-roformer-sw).
Licensing is deferred as a prototype selection gate at the user's request;
these downloads and another app's adoption do not establish distribution rights.

## Live terminal A/B mixer

Compare the two six-stem output directories at the same playback position. For
the existing whole-song BONES outputs, run from the repository root:

```sh
CARGO_HOME="$PWD/.cargo-home" cargo run --locked --release --example stem_mixer
```

Or point it at any compatible pair:

```sh
CARGO_HOME="$PWD/.cargo-home" cargo run --locked --release --example stem_mixer -- path/to/demucs-output path/to/roformer-output
```

It starts **paused**, with all six stems on. Press **Space** to listen.

| Key | Action |
| --- | --- |
| `1` | Mute/unmute bass |
| `2` | Mute/unmute drums |
| `3` | Mute/unmute guitar |
| `4` | Mute/unmute piano |
| `5` | Mute/unmute vocals |
| `6` | Mute/unmute other |
| `0` | Switch A/B at the current position |
| Space | Play/pause (restart from beginning at end) |
| Left / Right | Seek backward / forward five seconds |
| Esc / Ctrl+C | Quit |

The mute selection is shared across both models. To compare isolated guitar,
press `1`, `2`, `4`, `5`, `6`, then Space and `0`. For a backing-track comparison,
leave the others on and mute only `3`. Seeking clamps to the beginning/end;
displayed time is relative to the separated passage.

Each directory must contain `bass.wav`, `drums.wav`, `guitar.wav`, `piano.wav`,
`vocals.wav` and `other.wav`: nonempty stereo WAV, identical rate/frame count
across all twelve files, finite PCM. Float and integer PCM are accepted. If both
version-1 manifests exist, their source identity, passage and prepared-audio
identity must match, and their WAV filenames/geometry are checked. PCM hashes
are not recomputed. A missing manifest allows plain WAV banks with a warning
that source/passage identity is unverified. Geometry and matching provenance do
not establish perceptual onset alignment.

Both banks preload into memory for responsive seeking. Total raw PCM is capped
at 2 GiB; the BONES pair needs approximately 1.17 GiB, plus process/resampling
scratch. The app uses the default output device, prefers the stem rate when the
device supports stereo float there, and otherwise resamples each stem before
playback. One fixed common gain bounds the sum of absolute stem samples for
**both** banks and all mute combinations. This preserves A/B level differences
without per-model or per-stem normalization; adjust your system volume if the
conservative headroom makes playback quiet.

Mixing runs on a worker into a bounded 50 ms queue. The existing native callback
consumer applies paired 5 ms fades to transport/mix changes. Device buffers
already submitted cannot be recalled; switching is responsive but is not a
sample-accurate blind-test protocol. This standalone experiment has normal-speed
playback only and does not change desktop playback or saved songs. Terminal mode
is restored on normal exit and returned errors; an uncatchable termination may
require `reset` in the terminal.

For validation without opening an audio device, or a short audible hardware test:

```sh
CARGO_HOME="$PWD/.cargo-home" cargo run --locked --release --example stem_mixer -- --check
CARGO_HOME="$PWD/.cargo-home" cargo run --locked --release --example stem_mixer -- --smoke
CARGO_HOME="$PWD/.cargo-home" cargo test --locked --example stem_mixer
CARGO_HOME="$PWD/.cargo-home" cargo clippy --locked --example stem_mixer -- -D warnings
```

The four-second smoke plays through the default device and exercises A/B, mute
and seek. Callback counters establish device consumption, not listening quality.
