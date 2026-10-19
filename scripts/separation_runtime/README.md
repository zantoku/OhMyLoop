# Separation runtime build resources

These are maintainer/CI tools. The installed application carries uv, FFmpeg,
`worker.py`, target manifests and the small source-only wheels. It downloads a
managed interpreter and installs the exact wheel list locally, after an explicit
setup action. No code, recipes or runtime archives are fetched from the OhMyLoop
repository.

## Prepare a native build

Use Python 3.12 on the build machine. The Unix wheel build needs a native C
compiler; end users do not. Run from the repository root:

```sh
python3 scripts/separation_runtime/prepare.py --build-python /absolute/path/to/python3.12
npm run desktop:build
```

On this workspace the existing prototype interpreter is suitable for *building*:

```sh
.tools/stems/venv/bin/python scripts/separation_runtime/prepare.py --build-python .tools/stems/venv/bin/python
```

This command verifies the pinned upstream uv archive and imageio-ffmpeg wheel,
extracts their tools, builds Demucs 4.0.1 and (on Unix) diffq 0.2.4 from the pinned
source archives, and emits target-specific `requirements.lock` and
`bundled-artifacts.json`. On Windows, upstream diffq-fixed 0.2.4 supplies the
CPython 3.12 wheel. The native build runs with a pinned build-only wheel set,
`--no-build-isolation`, offline build execution and `SOURCE_DATE_EPOCH`.
Generated binaries, wheels, notices, build environments and runtime locks are
ignored by Git; the final installer includes them as Tauri resources.

`resources/separation/runtime/<target>/artifacts.json` is the reviewed acquisition
inventory. It records exact upstream URLs, hashes and lengths for interpreter,
bootstrap inputs and every inference distribution. `python-downloads.json` is a
single-entry uv metadata map, pinning Python **3.12.13 build 20260807**, not just
the interpreter version. The metadata key spells `darwin`; uv's installed
interpreter directory spells `macos`. `python_installation` records that mapping.

`definition.json` identifies the runtime recipe family. The application appends
a fingerprint of the packaged definition, generated lock, interpreter metadata,
artifact manifest and worker to form the effective immutable runtime ID. Changed
bundled wheels or worker code therefore build into a separate final directory;
the previous environment survives until verification succeeds. Identical payloads
reuse their installed runtime. This is not an independently updated package
channel. Do not move a populated virtual environment.

The Windows/Linux graph uses official PyTorch CPU wheels (`2.8.0+cpu`), never the
CUDA-capable Linux PyPI graph. `uv pip install` on the user's machine receives
only direct wheel references, `--no-deps`, `--no-index`, `--only-binary :all:` and
`--require-hashes`; it does not resolve dependencies or compile source.

## Refresh the reviewed artifact inventory

This is a deliberate maintainer operation, not part of ordinary builds:

```sh
.tools/stems/venv/bin/python scripts/separation_runtime/update-locks.py
```

It fetches metadata for the exact versions in the prototype's existing lock,
plus the explicitly pinned Windows additions and bootstrap providers. It uses
`packaging` for supported wheel tags; the prototype environment supplies that
module. It never selects latest versions. Review the diff and rerun native
assembly and packaged verification on **every** target after updating. A wheel's
architecture tag does not establish a universal CPU ISA compatibility floor.

## Installed-artifact verification

After a native installable build, use a fresh path:

```sh
python3 scripts/separation_runtime/packaged-smoke.py --root '/tmp/OhMyLoop packaged proof 雪'
```

Linux CI wraps that command in `xvfb-run -a`. The script copies the macOS app,
silently installs the Windows NSIS artifact, or extracts the Linux AppImage. It
sets application resources read-only on Unix, hides system tools through an empty
PATH, and invokes the packaged executable's `--stem-runtime-smoke` entry point.
It does not import runtime service code from this checkout. All song and setup
data is created under the selected fresh root; an existing root is refused.

Smoke logs and `results.json` distinguish successful modes from a job merely
being configured. The sequence tests cancelled setup, explicit setup, native
checks, short and normal generated-audio separation, cancellation, portability,
component removal and completed-stem use after removal. This is automated native
evidence, not hardware playback/listening, Gatekeeper approval or Windows
consumer-desktop coverage. A passing Windows Server runner does not prove
Windows 10/11 installation behavior. Signing/notarization provisioning is outside
this stage.

## Native requirements and audit boundaries

- macOS arm64: selected ONNX Runtime wheel requires macOS 14+. The first local
  bootstrap proof used the existing Mac, not a clean macOS 14 machine.
- Linux x64: selected native wheels require at least glibc 2.28. Desktop packaging
  is built/tested on Ubuntu 22.04; that does not claim every glibc 2.28 distro works.
- Windows x64: optional separation is conservatively gated to Windows 10+; the
  exact Windows build and consumer-desktop evidence must be recorded from actual
  tests. Native VC runtime availability must be exercised by packaged checks.
- Intel Mac, Windows ARM and Linux ARM inference are not selected targets.

The selected FFmpeg provider is imageio-ffmpeg **0.6.0**, whose binary versions
are **7.1** on macOS/Windows and **7.0.2** on Linux. The build script checks the
actual executable's version and records its complete build configuration.
The tested macOS binary links only system/Apple frameworks, not Homebrew paths.
The worker uses native Rust/SoundFile PCM and the direct Roformer demix path;
the upstream adapter nevertheless executes `ffmpeg -version` on construction.

The inherited `samplerate==0.1.0` universal wheel contains an Intel Mac library,
not an arm64 one. `import samplerate` already fails in the working arm64 prototype;
it is not imported by the selected Roformer path. This stage keeps that dormant
transitive distribution rather than changing the inference graph. Native readiness
and real inference exercise the libraries the selected adapter actually uses;
they are not a claim that all unused upstream model families work.

## Notices and source provenance

uv **0.12.16** is Apache-2.0 OR MIT. License texts from that exact tag are retained
in `licenses/uv-*` and copied into the shipped target notices. uv source:
<https://github.com/astral-sh/uv/tree/0.12.16>.

imageio-ffmpeg **0.6.0** is BSD-2-Clause; its wheel's notices are copied intact.
Its FFmpeg binaries are separate GPL-2.0-or-later builds (see executable `-L`).
GPL texts are retained in `licenses/FFmpeg-*`; the generated inventory records
all FFmpeg configure flags. Provider source and acquisition recipes:
<https://github.com/imageio/imageio-ffmpeg/tree/v0.6.0>;
FFmpeg sources: <https://ffmpeg.org/download.html>.

Demucs **4.0.1** carries MIT notices and diffq **0.2.4** carries
CC-BY-NC-4.0 notices; the build copies their source-provided licenses beside the
wheels, and preserves distribution metadata inside them. Other dependency wheel
licenses remain in their installed distribution directories. The acquisition
inventories record exact origins/hashes, and the generated bundled inventory
records the actual output-wheel hashes and tool sizes. These records preserve
provenance; they do not assert legal clearance for a public app release.
