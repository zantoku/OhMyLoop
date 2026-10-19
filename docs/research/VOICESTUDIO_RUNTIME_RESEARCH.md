# VoiceStudio v0.5.6: managed inference environments

Researched 2026-10-16 for OhMyLoop issue #7. Source review only: no upstream
setup, installation, inference or tests were executed. The annotated tag
`v0.5.6` (`e99127fbc759760311932cd465ae751c0370cbe9`) resolves to commit
**`3915a62cb482bb43117aaebab57d22745bd1cc21`**, dated 2026-09-23. All source
links below use that immutable commit; release metadata was separately inspected.

## Executive finding

**VoiceStudio implements real, user-facing environment setup in packaged apps.**
Its maintained desktop is Electron; Tauri ended at v0.5.3. It packages backend
sources, dependency manifests and uv, then explicitly downloads a managed Python
and installs locked dependencies on the user's machine. It does **not** deliver
a preassembled Python inference runtime. This is useful production-oriented
bootstrap machinery, rather than merely a developer setup script. [Desktop
boundary][readme] · [Bundle contents][bundle] · [Installer implementation][install]

At the time of this research, that central choice differed from
[ADR 0008](../adr/0008-isolated-separation-and-validated-artifacts.md), which required
a packaged runtime without on-device package-manager setup. On 2026-10-19 the
maintainer explicitly reopened that decision and chose local runtime assembly;
see the [current implementation plan](../STEM_RUNTIME_IMPLEMENTATION_PLAN.md).
The findings below describe the reviewed source, while its original recommendation
to retain preassembled delivery is historical rather than the current direction.

## Actual call path

| Boundary | Implemented path and evidence |
| --- | --- |
| First launch | `BackendSupervisor.start()` probes an existing backend; otherwise checks runtime compatibility/imports. Missing runtime yields `setup_required`, without downloading. [Startup][startup] |
| Explicit setup | Renderer `runSetup()` → preload bridge → trusted-main-frame IPC → `setupRuntime()` → `installRuntime()`. [UI][gate] · [IPC][ipc] · [Supervisor][setup] |
| Environment acquisition | Check destination write access and 9 GiB free; stage bundled sources; use bundled/found uv, with an app-private installer fallback; run frozen dependency synchronization with managed Python 3.11. [Installation][install] |
| Runtime readiness | Reject an installation-in-progress marker; compare dependency stamp; execute imports using the selected interpreter. Successful setup writes readiness last. [Checks][ready] · [Final verification][finish] |
| Backend execution | Launch the environment's Python directly as `-m uvicorn main:app --app-dir backend --host 127.0.0.1`; no synchronization during ordinary startup. [Spawn plan][spawn] |
| Weights | Separate model-install HTTP action runs background downloads with curated immutable repository revisions, then reports model readiness. [Download][models] · [Revision registry][revisions] |
| Inference | Backend engine adapters perform inference; default OmniVoice uses an additional subprocess on MPS, but runs inside the backend on CPU/CUDA/ROCm. Other incompatible engines have separate managed environments. [Isolation][inference] · [Engine installer][sidecars] |

These are connected implementation paths with regression and packaged-smoke code,
not just a proposed architecture. This review does not establish their success on
fresh machines.

## Runtime contents, isolation and platform constraints

The release workflow pins **uv 0.12.13**, copies the host-architecture executable
into resources, and checks packaged resources. Python is selected as **3.11**, not
an exact patch/build. `uv sync --frozen --no-dev` fixes the dependency graph; it
does not make the interpreter selection equally precise. [Release assembly][release-ci]
· [Python selection][finish] · [Packaging contract][contract]

Relevant locked dependencies include torch/torchaudio **2.8.0** (CUDA **+cu128**
variants on Linux/Windows x64), transformers **5.15.1**, huggingface-hub **1.28.0**,
SentencePiece **0.2.2**, faster-whisper **1.2.1**, and imageio-ffmpeg **0.6.0**.
The broad multi-engine graph includes CUDA wheels even for CPU use on those
platforms. This is substantially larger than our selected Roformer-SW CPU-only
need. [Platform indexes and constraints][torch] · [Resolved lockfile][lock]

Readiness probes clear `PYTHONHOME`/`PYTHONPATH`, disable user-site imports and
request Hugging Face offline behavior. They import real native dependencies,
including SentencePiece, rather than trusting package metadata. The SHA-256
readiness stamp hashes manifests and a schema identifier; **it is a compatibility
stamp, not an integrity inventory of installed files**. [Readiness implementation][ready]
· [Stamp calculation][stamp]

Native compatibility remains work: CUDA setup adds pinned cuDNN 8.9.7.29 libraries
beside the main environment, while Linux patches CTranslate2's executable-stack
request. FFmpeg comes first from an override or imageio-ffmpeg's platform binary;
FFprobe needs another source. Their media-tools service can acquire exact-size,
SHA-256-pinned static archives and verifies executable usability. This is a useful
pattern for auditing transitive executable needs, not evidence that every binary
is included in the desktop installer. [Compatibility code][native]
· [Tool resolution][ffmpeg] · [Pinned media archives][media] · [Download verification][media-download]

The configured delivery matrix is Windows x64, Linux x64, and macOS arm64/x64.
**An Intel Mac installer is not local inference support:** preview CI explicitly
skips runtime installation there because required PyTorch wheels are unavailable.
There is no corresponding Linux/Windows ARM desktop release matrix in these
workflows. [Matrix][preview-ci]

## Storage, retry, updates and cleanup

The default environment is `<Electron userData>/runtime/project`, with a selectable
custom root. `.venv` and staged code live in `project`; sibling `.python` and
`.uv-cache` preserve interpreter and dependency downloads across **Clean & Retry**.
Model caches are separate: normal Hugging Face paths or explicit overrides;
Windows selects a shorter app-specific default to avoid path-length failures.
Outputs, voices and the database have another application-data root.
[Runtime paths][paths] · [Cache isolation][install] · [Model/data paths][data]

Setup records `.runtime-installing` before mutation and clears it only after
verification; interruption therefore cannot masquerade as readiness. Retry reuses
downloads. Clean repair removes only the owned project, refusing unowned custom
or reused environments. Compatible manifests allow bundled code replacement
without discarding dependencies; changed graphs return to explicit setup.
These environment markers are separate from song-publication journaling, which
OhMyLoop excluded. [Markers and staging][ready] · [Owned repair][repair]

The uninstall planner distinguishes shared models, environment and user data,
validates target roots, and refuses inappropriate paths. This is a useful cleanup
ownership model, not an invitation to copy their migration/history machinery.
[Uninstall planning][uninstall]

## Downloads, offline behavior and process lifetime

Curated model downloads use immutable revisions. The segmented downloader checks
length and, when the supplied ETag is a SHA-256, hashes bytes before replacement.
However, optional sidecar recipes can track upstream sources/default-branch
weights; the entire catalog is not governed by one uniform frozen contract.
[Curated pins][revisions] · [Byte verification][segmented]
· [Sidecar weight policy][sidecar-weights]

The uv fallback downloads version-addressed installer text over HTTPS, bounds it
to 2 MiB/60 seconds/five redirects and executes it; this call path has no separate
expected installer hash. Normal releases bundle uv and avoid that fallback.
Python/package acquisition still needs network access. [Fallback][install]
· [Download bounds][runtime-download]

**Local inference is not a blanket offline guarantee.** Runtime installation,
weights and media tools have separate acquisition paths. A damaged model load can
automatically re-fetch weights unless offline flags are set. Some engine bootstrap
also happens while resolving a sidecar interpreter. Our explicit model acquisition
and strictly offline worker should remain explicit choices. [Load-time repair][auto-repair]
· [Offline repair guard][repair-offline]
· [Sidecar resolution][subprocess]

Cancellation is also path-dependent: runtime cancellation aborts downloads and
terminates the owned setup/backend process tree; Hugging Face fallback downloads
cannot necessarily stop mid-file, so model cancellation retains ownership until
the worker returns. POSIX shutdown signals a process group, escalating after two
seconds; Windows invokes `taskkill /T /F`. A parent-liveness pipe additionally
retires an orphaned backend. Their Windows implementation documents a native-import
deadlock caused by a blocking pipe reader and uses polling instead. These details
justify real packaged lifecycle tests. [Shutdown][shutdown] · [Model cancel][cancel]
· [Parent watcher][parent]

## What release evidence actually establishes

The [v0.5.6 release][release] contains real Electron assets: macOS arm64 DMG
266,952,331 bytes, Windows x64 installer 214,447,484, Linux AppImage 273,771,944,
and deb 204,482,888. These sizes exclude later environment/model downloads. The
release explicitly discloses unsigned/ad-hoc-signed, unnotarized installers and
unverified macOS automatic updates. No installers were downloaded or exercised here.

Preview CI config runs a fresh packaged `--install` smoke on supported local-runtime
targets. Release CI runs `--setup`, which verifies consent gating without installing.
The `--install` smoke installs dependencies and reaches backend readiness with
empty isolated HF cache/offline flags; `--first-sound` is a distinct networked model
and generation test. Configured jobs are not evidence those jobs passed.
[Preview jobs][preview-ci] · [Release jobs][release-ci] · [Smoke boundaries][smoke]

Versioned runtime notes report successful Windows/WSL installation and relaunch,
but describe backend **0.5.2**, and explicitly leave fully uncached macOS
first-sound verification outstanding. Treat this as maintainer-reported historical
evidence, not an independently verified v0.5.6 cross-platform release result.
[Reported verification and limits][reported]

## Lessons and issue #7 decision points

**Recommendations/inferences for OhMyLoop:**

- Adopt separate runtime readiness/model installation states, exact manifest
  identity plus real native-import probes, app-owned cache boundaries, and
  disposable-environment repair that preserves completed stem sets.
- Adopt isolated packaged smoke tests with empty caches, workspace/system-Python
  hidden, subsequent offline inference, cancellation/quit and restart. Assert
  actual audio results as well as backend readiness.
- Keep our single CPU2 model package narrow. Their multi-engine CUDA graph,
  repository cloning, dynamic package installation and catalog repair complexity
  address a different product scope.
- Preserve provenance and third-party notices for whatever runtime we distribute;
  the source bundle includes LICENSE and separately documents component terms.
  This is provenance work, not a new licensing gate. [Bundle][bundle] · [Notices][notices]

Before refining #7, decide: retain preassembled runtime delivery or reopen ADR 0008
for managed bootstrap; include the runtime in the installer or acquire a separate
prebuilt package; supported OS/architecture/minimum CPU matrix; exact interpreter
build/dependency identity; runtime update/repair/removal ownership; and the concrete
clean-machine/offline/signing evidence required per platform. VoiceStudio does not
settle those decisions for us.

**Suggested first packaging experiment:** retain the current ADR initially and
assemble one macOS arm64 runtime at build time from OhMyLoop's pinned Python and
CPU dependency graph. Bundle it with the existing worker and required native
executables, then run a relocated installed app from a clean profile with network
blocked after explicit weights acquisition. Compare generated stem geometry/audio,
measure installed/download sizes and startup, and exercise cancel/quit. This tests
the missing relocation/native-library boundary directly. VoiceStudio's source
provides the acceptance-test pattern, not proof that an assembled venv is relocatable.

[readme]: https://github.com/debpalash/VoiceStudio/blob/3915a62cb482bb43117aaebab57d22745bd1cc21/README.md#L89-L105
[bundle]: https://github.com/debpalash/VoiceStudio/blob/3915a62cb482bb43117aaebab57d22745bd1cc21/electron/electron-builder.config.mjs#L26-L85
[startup]: https://github.com/debpalash/VoiceStudio/blob/3915a62cb482bb43117aaebab57d22745bd1cc21/electron/src/main/backend.ts#L480-L535
[gate]: https://github.com/debpalash/VoiceStudio/blob/3915a62cb482bb43117aaebab57d22745bd1cc21/electron/src/renderer/src/components/backend-gate.tsx#L142-L150
[ipc]: https://github.com/debpalash/VoiceStudio/blob/3915a62cb482bb43117aaebab57d22745bd1cc21/electron/src/main/ipc.ts#L481-L495
[setup]: https://github.com/debpalash/VoiceStudio/blob/3915a62cb482bb43117aaebab57d22745bd1cc21/electron/src/main/backend.ts#L544-L670
[install]: https://github.com/debpalash/VoiceStudio/blob/3915a62cb482bb43117aaebab57d22745bd1cc21/electron/src/main/runtime-project.ts#L354-L425
[ready]: https://github.com/debpalash/VoiceStudio/blob/3915a62cb482bb43117aaebab57d22745bd1cc21/electron/src/main/runtime-project.ts#L235-L352
[stamp]: https://github.com/debpalash/VoiceStudio/blob/3915a62cb482bb43117aaebab57d22745bd1cc21/electron/src/main/runtime-project.ts#L85-L89
[finish]: https://github.com/debpalash/VoiceStudio/blob/3915a62cb482bb43117aaebab57d22745bd1cc21/electron/src/main/runtime-project.ts#L426-L476
[spawn]: https://github.com/debpalash/VoiceStudio/blob/3915a62cb482bb43117aaebab57d22745bd1cc21/electron/src/main/backend.ts#L286-L310
[models]: https://github.com/debpalash/VoiceStudio/blob/3915a62cb482bb43117aaebab57d22745bd1cc21/backend/api/routers/setup/download.py#L590-L625
[revisions]: https://github.com/debpalash/VoiceStudio/blob/3915a62cb482bb43117aaebab57d22745bd1cc21/backend/services/hf_revisions.py#L1-L102
[inference]: https://github.com/debpalash/VoiceStudio/blob/3915a62cb482bb43117aaebab57d22745bd1cc21/backend/engines/omnivoice_subprocess/__init__.py#L1-L27
[sidecars]: https://github.com/debpalash/VoiceStudio/blob/3915a62cb482bb43117aaebab57d22745bd1cc21/backend/services/sidecar_install.py#L1-L44
[release-ci]: https://github.com/debpalash/VoiceStudio/blob/3915a62cb482bb43117aaebab57d22745bd1cc21/.github/workflows/electron-release.yml#L114-L180
[contract]: https://github.com/debpalash/VoiceStudio/blob/3915a62cb482bb43117aaebab57d22745bd1cc21/electron/tests/packaging-contract.mjs#L69-L85
[torch]: https://github.com/debpalash/VoiceStudio/blob/3915a62cb482bb43117aaebab57d22745bd1cc21/pyproject.toml#L275-L307
[lock]: https://github.com/debpalash/VoiceStudio/blob/3915a62cb482bb43117aaebab57d22745bd1cc21/uv.lock#L6788-L7101
[native]: https://github.com/debpalash/VoiceStudio/blob/3915a62cb482bb43117aaebab57d22745bd1cc21/electron/src/main/runtime-project.ts#L145-L231
[ffmpeg]: https://github.com/debpalash/VoiceStudio/blob/3915a62cb482bb43117aaebab57d22745bd1cc21/backend/services/ffmpeg_utils.py#L193-L320
[media]: https://github.com/debpalash/VoiceStudio/blob/3915a62cb482bb43117aaebab57d22745bd1cc21/backend/services/media_tools.py#L12-L36
[media-download]: https://github.com/debpalash/VoiceStudio/blob/3915a62cb482bb43117aaebab57d22745bd1cc21/backend/services/media_tools.py#L199-L230
[preview-ci]: https://github.com/debpalash/VoiceStudio/blob/3915a62cb482bb43117aaebab57d22745bd1cc21/.github/workflows/electron-build.yml#L20-L122
[paths]: https://github.com/debpalash/VoiceStudio/blob/3915a62cb482bb43117aaebab57d22745bd1cc21/electron/src/main/backend.ts#L244-L275
[data]: https://github.com/debpalash/VoiceStudio/blob/3915a62cb482bb43117aaebab57d22745bd1cc21/backend/core/config.py#L5-L71
[repair]: https://github.com/debpalash/VoiceStudio/blob/3915a62cb482bb43117aaebab57d22745bd1cc21/electron/src/main/backend.ts#L674-L719
[uninstall]: https://github.com/debpalash/VoiceStudio/blob/3915a62cb482bb43117aaebab57d22745bd1cc21/electron/src/main/uninstall-data.ts#L56-L113
[segmented]: https://github.com/debpalash/VoiceStudio/blob/3915a62cb482bb43117aaebab57d22745bd1cc21/backend/services/segmented_download.py#L199-L216
[sidecar-weights]: https://github.com/debpalash/VoiceStudio/blob/3915a62cb482bb43117aaebab57d22745bd1cc21/backend/services/sidecar_install.py#L1657-L1688
[runtime-download]: https://github.com/debpalash/VoiceStudio/blob/3915a62cb482bb43117aaebab57d22745bd1cc21/electron/src/main/runtime-download.ts#L46-L95
[auto-repair]: https://github.com/debpalash/VoiceStudio/blob/3915a62cb482bb43117aaebab57d22745bd1cc21/backend/services/model_manager.py#L2469-L2497
[repair-offline]: https://github.com/debpalash/VoiceStudio/blob/3915a62cb482bb43117aaebab57d22745bd1cc21/backend/services/model_manager.py#L2145-L2171
[subprocess]: https://github.com/debpalash/VoiceStudio/blob/3915a62cb482bb43117aaebab57d22745bd1cc21/backend/services/subprocess_backend.py#L502-L533
[shutdown]: https://github.com/debpalash/VoiceStudio/blob/3915a62cb482bb43117aaebab57d22745bd1cc21/electron/src/main/backend.ts#L1147-L1182
[cancel]: https://github.com/debpalash/VoiceStudio/blob/3915a62cb482bb43117aaebab57d22745bd1cc21/backend/api/routers/setup/download.py#L984-L1026
[parent]: https://github.com/debpalash/VoiceStudio/blob/3915a62cb482bb43117aaebab57d22745bd1cc21/backend/core/parent_liveness.py#L34-L85
[release]: https://github.com/debpalash/VoiceStudio/releases/tag/v0.5.6
[smoke]: https://github.com/debpalash/VoiceStudio/blob/3915a62cb482bb43117aaebab57d22745bd1cc21/electron/tests/packaged-smoke.mjs#L45-L77
[reported]: https://github.com/debpalash/VoiceStudio/blob/3915a62cb482bb43117aaebab57d22745bd1cc21/docs/electron-runtime.md#L57-L61
[notices]: https://github.com/debpalash/VoiceStudio/blob/3915a62cb482bb43117aaebab57d22745bd1cc21/LICENSE-NOTICE.md#L32-L51
