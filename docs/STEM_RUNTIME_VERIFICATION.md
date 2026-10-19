# Managed separation runtime verification (#7)

Implementation and evidence collected 2026-10-19. The accepted scope is
[the runtime brief](STEM_RUNTIME_IMPLEMENTATION_PLAN.md). **Issue #7 remains open
until actual installed-app gates pass on all three selected targets.** Workflow
configuration and a local bootstrap experiment do not satisfy that gate.

## Reproduce

Use a native target and Python 3.12 for maintainer-side resource assembly:

```sh
python3 scripts/separation_runtime/prepare.py --build-python /absolute/path/to/python3.12
export CARGO_HOME="$PWD/.cargo-home"
npm ci
npm run desktop:build
python3 scripts/separation_runtime/packaged-smoke.py --root '/absolute/fresh/OhMyLoop proof 雪'
```

The driver refuses an existing root. It copies the Mac `.app`, silently installs
the Windows NSIS artifact, or extracts the Linux AppImage. Unix resources become
read-only; system Python, compiler and FFmpeg are hidden with an empty application
PATH. The app resolves its own installed resources and calls the production
manager/controller through `--stem-runtime-smoke`, rather than loading a workspace
worker. Linux CI runs this under `xvfb-run -a` for native GUI initialization.

The suite exercises setup cancellation, shutdown during setup, explicit retry,
completion, native readiness after cache cleanup and process restart, 1-second
and 20-second generated audio, cancellation and shutdown during inference,
closed-library move/copy/reopen, component removal, and completed-stem selection
without tools/model. Authored notes, a nonempty measure map, sections, controls,
reference geometry and recording/stem bytes are checked. Setup progress is
indeterminate where uv exposes no reliable byte denominator. Model byte progress
is genuine; Ready does not start separation.

After setup the driver temporarily renames **only its scratch copy** of uv and
sets acquisition proxies to an unreachable loopback port. Native checks and
inference must still work. The worker additionally forbids socket connections.
This tests no dependency synchronization/download during reuse; it is not a claim
of an operating-system network sandbox. The uv filename is restored on exit.

Tests invoke the same production shutdown used after the desktop's confirmed quit;
frontend tests separately verify confirmation and setup/separation availability.
No journal, resume, recovery UI or persistent diagnostics were added.

## Local evidence

Host: Apple M5 Pro, 48 GiB RAM, macOS 26.6.2, arm64. Native/bootstrap checks and
installed smoke logs are under `/tmp/managed-runtime-bootstrap-proof.log` and
`/tmp/ohmyloop-installed-smoke.log`; per-mode logs and `results.json` are in
`/private/tmp/OhMyLoop installed proof 雪 20261019/`.

- Maintainer preparation built only missing Demucs and diffq wheels; the user-side
  setup installed 68 exact hash-locked binary distributions without a compiler.
- Actual Mac release `.app` and `.dmg` built. Initial restricted-sandbox disk image
  creation failed; the authorized build outside that filesystem restriction passed.
  Signing/notarization provisioning remains outside #7; no public release published.
- Relocated read-only-app setup cancellation passed (3.171 s including app startup).
- Explicit retry and complete tools/model setup passed: 196.323 s measured inside
  the app; 196.953 s including app startup/shutdown. This run followed the deliberate
  cancelled attempt and reused its interpreter; it is not an entirely uncached
  first-ever network benchmark. Filesystem sampling adds measurement overhead.
- After setup: 1,597,097,427 bytes app data, sampled peak 1,707,769,442 bytes.
  Runtime's pinned upstream payload is 242,918,444 bytes; model payload is
  699,416,805 bytes. Installer-bundled tools/wheels are separate from those downloads.
- Restarted native readiness/cache-clean independence passed (1.659 s).
- Generated 1-second real separation passed (38.462 s including app startup).

The strengthened shutdown/offline/portability suite and full-song resource results
are recorded as they complete. Do not infer their success from earlier smoke modes.

## Platform evidence matrix

| Target | Artifact-imposed runtime floor | Evidence currently recorded | Remaining gate |
| --- | --- | --- | --- |
| macOS arm64 | macOS 14+, selected ONNX Runtime wheel | macOS 26.6.2 bootstrap, installer build, relocated setup, native checks, short inference above | Strengthened complete suite and full-song measurements |
| Windows x64 | Windows 10+; exact native requirements tested by probe | Pinned recipe and Windows `NOMINMAX` build repair configured | Actual NSIS installed setup/inference/offline/cancellation run |
| Linux x64 | glibc 2.28+; desktop tested on Ubuntu 22.04 | Pinned recipe and AppImage workflow configured | Actual extracted AppImage setup/inference/offline/cancellation run |

Windows Server CI is not Windows consumer-desktop validation. A selected wheel's
architecture tag does not prove an AVX/AVX2 floor. Intel Mac and Windows/Linux ARM
inference remain unsupported; existing playback and completed stems remain usable.
No new hardware playback or subjective listening claim follows from headless QA.

## Checks and review

Native release tests with `beat-analysis,stem-separation`, strict workspace/all-target
Clippy, frontend tests, Svelte checks and production frontend build passed locally.
Managed setup regressions cover failure/retry/cache preservation, native failure,
same-base-ID recipe replacement, safe removal, process-tree cancellation, and
post-publication retirement failure preserving the new verified runtime.
Independent standards/spec review findings are resolved as documented in the
implementation handoff; platform verification still governs issue closure.

See [build resources and provenance](../scripts/separation_runtime/README.md) and
[safe cleanup](STEM_DATA_CLEANUP.md). Model terms are not reinterpreted by this work;
preserved notices and pinned origins are provenance, not public-release clearance.
