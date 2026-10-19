# Issue #7: managed separation runtime setup

Status: scope confirmed and implementation authorized by the maintainer on
2026-10-19. This brief and GitHub issue #7 govern implementation and review.
Origin: [GitHub issue #7](https://github.com/zantoku/OhMyLoop/issues/7).

## Confirmed direction

- Keep Roformer-SW and the existing fixed CPU/two-thread separation behavior.
- Assemble the separation runtime on the user's machine. The maintainer reversed
  the preassembled strategy on 2026-10-19: accept more setup complexity to avoid
  project-operated runtime hosting and dependence on the OhMyLoop repository's
  location. The installed app must carry its setup recipe and worker rather than
  fetching them from the repository. Do not depend on system Python or workspace
  scripts. Preserve explicit model acquisition and offline inference after setup.
- Start with a macOS arm64 managed-setup proof: relocate the installed app, hide
  workspace/system Python and verify setup, actual separation, cancellation and quit.
  Extend the proven approach to the supported platform targets.
- Runtime packages, model packages and completed song/stem data have distinct
  identities and lifecycles. Existing stems continue playing independently of
  inference tools.
- Use VoiceStudio's source-backed managed-bootstrap, readiness, ownership and
  packaged-smoke lessons while retaining our narrower CPU-only model scope. See
  [the research](research/VOICESTUDIO_RUNTIME_RESEARCH.md).

## Confirmed interview decisions: Q1–Q4

- **Q1 — Targets:** macOS Apple Silicon, Windows x64 and Linux x64, with the
  macOS arm64 proof first. Defer Intel Mac and Windows/Linux ARM inference.
  Separation-runtime support is distinct from support for the rest of the app.
  Include necessary Windows native-build repairs to verify that target. Exact
  minimum OS/native requirements must be derived from the selected artifacts and
  recorded alongside actual tested versions; do not imply broader coverage.
- **Q2 — Delivery, superseded:** the previously accepted preassembled-runtime
  download is replaced by explicit local assembly. No OhMyLoop runtime archives
  will be hosted. Q10–Q15 below establish the replacement mechanism and flow.
- **Q3 — Compatibility/updates:** each app release selects an exact compatible
  runtime definition. Reuse it when installed; explicitly set up any required
  replacement. No independent runtime-update channel or version chooser. An app
  update need not require a new runtime if the selected package remains compatible.
- **Q4 — Distribution boundary:** certificate provisioning, public app release
  publication and automatic app updates are outside this implementation stage.
  Build/test installable artifacts and document their actual signing status.
  Runtime-artifact hosting/publication is no longer part of this stage.

## Cost constraint

Do not host runtime packages or introduce paid hosting/CDNs/distribution storage.
Do not enable paid build infrastructure or raise billing limits as part of this
work. The GitHub runtime-prerelease proposal (Q8) is withdrawn.

Repository independence does not establish upstream independence. Fresh setup
will need externally supplied interpreter/dependency artifacts unless they are
included in the installer. Q10–Q12 confirm pinned third-party acquisition with
verification and no permissive fallback. Model-weight
hosting is a separate existing acquisition path; this change does not silently
move or replace it.

## Decision history

Q1, Q3 and Q4 remain accepted with the terminology changes above. Q2 is superseded;
Q8 is withdrawn. Q10–Q15 now replace the previously unanswered Q5 (combined setup
flow), Q6 (app-data storage), Q7 (repair/removal) and Q9 (completion evidence) with
accepted decisions for local assembly. The old ban on on-device dependency
installation no longer applies.

All decisions asked through Q20 are accepted, superseded or withdrawn as recorded
here. Do not silently inherit obsolete model selection, concurrent playback during
separation, runtime hosting or journaling/recovery scope from the original issue.
Exact native artifact closure and supported-version evidence are implementation
investigations with the fixed constraints below, not permission to change them.

## Confirmed revised decisions: Q10–Q15

Accepted together by the maintainer on 2026-10-19.

- **Q10 — Independence boundary:** no dependency on the OhMyLoop repository at
  runtime/setup; allow pinned third-party downloads for fresh setup/repair.
- **Q11 — Bootstrap prerequisites:** bundle pinned uv, worker, setup manifests and
  FFmpeg; assemble app-private Python/dependencies locally, without system Python,
  an end-user compiler, package-manager prerequisites or administrator setup.
  If necessary, bundle a narrowly scoped set of missing dependency wheels rather
  than an entire preassembled environment. Dependency changes require proof.
- **Q12 — Acquisition policy:** exact per-target artifacts and approved sources;
  verify downloaded artifacts and fail usefully if unavailable or invalid. No
  fallback to latest versions, alternative indexes or source compilation.
- **Q13 — Setup flow:** one explicit Set up stem separation action, visible
  download/install/check steps, playback available during setup, Ready on success,
  and a separate Separate song action. No dependency installation on ordinary launch.
- **Q14 — Ownership:** shared app-data runtime, no folder picker; explicit tool
  reinstall/removal preserves models and all library data. Controls unavailable
  while the relevant assets are in use. Q16–Q18 settle interruption/cache/replacement.
- **Q15 — Evidence:** successful installed-app setup, native readiness, actual
  separation, offline reuse and cancellation on all three selected targets before
  closing #7; report hardware/listening evidence separately and honestly.

## Confirmed lifecycle decisions: Q16–Q20

Accepted together by the maintainer on 2026-10-19.

- **Q16 — Interrupted setup:** cancel stops/reaps owned setup processes before
  cleanup. Failed/interrupted environments are not Ready; explicit retry recreates
  the incomplete environment, reusing verified downloads where possible. No
  dependency-installation resume, journal or automatic restart-time repair.
  Closing the screen preserves setup; quitting uses existing confirmation and
  cancel/stop behavior. Independently verified runtime/model components survive.
- **Q17 — Download cache:** keep reusable downloads after failure/cancellation;
  clear disposable runtime download caches after successful setup to avoid keeping
  a second copy. Keep the managed interpreter. Tool removal includes its owned
  download cache. No cache-management screen; repair may download again.
- **Q18 — Runtime replacement:** build a required replacement in its own final
  versioned directory. Retain the old runtime until verification succeeds, then
  remove it; no runtime history/rollback UI. Check estimated free space once before
  setup, allowing for temporary overlap. An incompatible old runtime does not
  satisfy the new app's readiness requirement.
- **Q19 — Runtime readiness:** quick compatibility/presence checks for ordinary
  status; bounded real native/FFmpeg/CPU checks after setup and before separation.
  No full environment-file hash scan on every launch or background watcher.
  Keep existing model checksum checks. Failed runtime probes require explicit
  repair and never silently install or download.
- **Q20 — Resource/portability scope:** measure fresh setup download/installed/peak
  disk sizes, setup duration, and a full-song Mac separation's elapsed time and
  peak memory. Verify existing closed-app library move/copy/reopen behavior;
  document actual results and limits. No resource dashboard, new library recovery
  features or performance optimization project in #7.

## Implementation work packages

### 1. Runtime definition and compiler-free artifact closure

Create a versioned, validated per-target definition for the exact interpreter
build, CPU dependency artifacts, approved sources, lengths/checksums where
available, bundled tools/wheels, worker/protocol compatibility and native
requirements. Do not resolve dependencies dynamically on the user's machine.
Use the known Python 3.12.13/Roformer-SW graph as the starting point; account
explicitly for platform wheel identities such as PyTorch's `+cpu` suffix in
readiness and result provenance. Preserve preprocessing, inference settings,
source-frame geometry, role validation and the CPU2 profile.

Prove the Demucs/diffq wheel gap is resolved without an end-user compiler. Prefer
a narrow, evidenced solution; do not turn this into broad dependency pruning or
model replacement. Build any necessary small bundled wheels during maintainer/CI
assembly, with pinned inputs, hashes, native compatibility checks and reproducible
instructions. Do not commit generated environments, tools, caches or builds.
Audit bundled native tools/libraries and preserve applicable notices/provenance.

Assemble a macOS arm64 installed-app proof first. If the fixed prerequisites
cannot be met, report the specific blocker rather than silently requiring system
Python, developer tools, a paid host, different inference settings or another model.

### 2. Managed setup service and ownership

Keep the runtime definition, setup execution, persistence/readiness and UI
presentation separate. Extend the existing separation manager through a cohesive
setup service rather than growing one event handler. Inject app resource and
app-data paths from the desktop boundary; shipped code must not resolve resources
through `CARGO_MANIFEST_DIR` or a workspace fallback.
Preserve the existing prototype/CLI and its tests. Keep any development-runtime
mode explicit and separate from the installed-app path; a packaged app must
demonstrate managed setup even on a developer machine with a working prototype.

Bundle the selected uv executable, FFmpeg, worker and setup definitions as Tauri
resources. Use the bundled uv to acquire the exact managed interpreter and install
the selected wheels in an app-owned final versioned directory. Virtual environments
are created at their final path, not moved after creation. Keep the managed
interpreter separate from the disposable download cache; retiring a runtime must
not delete an interpreter still required by the selected installation.

Control child environment, approved package sources, managed-only Python selection,
native executable/library lookup and writable caches explicitly. Disable inherited
Python/package-manager configuration, user-site imports and arbitrary system-tool
fallback. Execute installed Python directly for probes/inference; ordinary launch
and inference must not invoke dependency synchronization or acquire files.

Publish a small compatibility/completion record only after successful verification.
Missing or incompatible completion records do not count as Ready. Retry recreates
the recognized incomplete environment after owned processes have stopped. This
record is not a journal or dependency-installation resume mechanism. Guard deletion
to app-owned tool/cache paths; never remove model or library data as runtime repair.

Setup and separation must not overlap or race runtime/model removal. Extend the
existing owned process-tree cancellation and shutdown guarantees to uv and probes.
Keep attempt IDs, progress and diagnostics bounded and reject stale completions.
Leave playback available during setup and preserve the existing separation-time
freeze policy. Do not add filesystem/process work to the audio callback.

### 3. Setup and management UI

Replace the model-only setup entry with **Set up stem separation**. Show runtime
readiness separately from model readiness so existing verified components can be
reused. A fresh setup assembles/verifies tools and acquires the missing model;
failure or cancellation preserves independently completed components. A tool-only
repair must not redownload a verified model.

Reuse the established progress/checklist components and their accessible,
scroll-following behavior. Use short descriptions for download/install/check
steps, byte progress where reliable, and indeterminate progress for work without
a trustworthy denominator. Do not invent elapsed-time percentages or reuse the
separation inference weights for setup. Retain twelve-o'clock clockwise pies.
Ready appears only when the required runtime and model are ready; **Separate song**
remains a separate explicit action. Keep setup and separation progress titles
appropriate to their respective workflows.

Provide retry, cancellation, **Reinstall separation tools** and **Remove separation
tools** alongside existing model management. Explain that setup needs downloads,
while installed separation works offline. Keep current-attempt failure details
available without persistent diagnostic history. Screen closure does not cancel
setup; reopening displays the current attempt. Relevant management controls are
unavailable while assets are in use; quit waits for cancellation/cleanup as agreed.

### 4. Packaged platform verification and documentation

After the Mac proof, build installable Windows x64 and Linux x64 artifacts and
exercise their managed setup. Include the existing Windows native-build repair
needed for this gate. Pin explicit CI OS/architecture targets instead of deriving
support promises from moving `latest` labels. Record artifact-imposed requirements,
tested OS/CPU, runner resources and signing status separately. Do not infer consumer
Windows coverage solely from a Windows Server CI run, or a minimum CPU ISA solely
from a wheel's architecture tag.
Validate known runtime OS/architecture requirements before acquiring dependencies.
Unsupported separation targets must leave completed-stem playback and the rest of
the existing app usable. Do not silently raise the whole desktop app's minimum
OS solely to match one optional inference dependency.

Run actual installed-app setup and inference with workspace/system Python and
system FFmpeg hidden, fresh app-owned caches and no privileged dependency setup.
Use paths containing spaces/non-ASCII characters and read-only app resources.
Once setup completes, run again with acquisition unavailable and verify both
separation and completed-stem playback. Remove runtime/model components separately
and verify completed audio remains playable.

Use generated audio for automated fixtures; verify real produced audio and exact
reference geometry, not merely process exit or dependency metadata. Exercise a
short input and a representative normal input on each target. Keep hosted-runner
memory limits in mind; the full-song memory/timing measurement is on the available
Mac, not a requirement to fit that song into every free CI VM.

Document fresh setup bytes/duration, installed/peak disk, runtime replacement's
temporary overlap, full-song Mac separation time/peak memory, supported/tested
matrix, offline limitations, cleanup/repair commands and signing status. Preserve
closed-app whole-library copy/move/reopen and manual backup instructions; verify
recordings, maps, notes and completed stems survive that workflow. Do not add a
migration/recovery feature to make this test pass.

## Acceptance and review gates

- All selected targets produce installable artifacts and pass actual setup,
  native readiness, real separation, offline reuse and cancellation tests.
  Configured CI without a passing run does not close #7.
- Setup succeeds without system Python/FFmpeg, a compiler, administrative setup,
  OhMyLoop repository access or hosted OhMyLoop runtime archives. Third-party
  setup downloads and the separate existing model sources remain allowed.
- Tests cover unavailable/invalid artifacts, incomplete setup, explicit retry,
  cancellation/quit with process reaping, stale attempt results, native-probe
  failure, compatible reuse, replacement and safe tool removal. Verified components
  and all song data survive failed attempts. No failure starts an implicit download.
- Test cache cleanup followed by offline native checks and inference; this catches
  installed dependencies accidentally linked to deleted cache files.
- UI tests cover missing-tool/model combinations, retained components after retry,
  progress and management availability; visually exercise fresh setup, cancellation,
  reopening the screen and Ready without automatic separation.
- Run formatting, strict linting, focused native/frontend tests, existing audio
  invariants and desktop builds. Document exact commands and actual results.
- Review standards and agreed scope independently, then resolve findings before
  handoff. Hardware playback/listening evidence is reported separately; do not
  claim it for targets tested only through automation.

## Explicit exclusions

No runtime hosting/publication, paid infrastructure, public app release/signing
provisioning, updater, GPU/model selection, Intel Mac/Windows ARM/Linux ARM inference,
runtime folder picker/version chooser, history/rollback UI, journal/resume/automatic
repair, persistent diagnostics, resource dashboard, library migration/recovery or
new export features. Broad obsolete-prototype/documentation cleanup remains #8;
update only documentation made inaccurate by #7.

## Initial technical findings

The existing workspace environment is about 955 MiB installed, excluding weights
and FFmpeg (883 MiB venv plus 72 MiB interpreter); its virtual
environment contains absolute workspace references and is not a shipping artifact.
Current CI builds unbundled executables rather than testing installed app bundles.
The current flat development dependency list does not establish locked dependency
closure across all target platforms. Readiness must verify native imports as well
as package metadata. Local assembly removes the need to relocate a prebuilt venv,
but the installed app and its resource paths must remain independent of the workspace.

- Existing selected macOS dependencies include ONNX Runtime 1.31.0 with a macOS
  14+ arm64 wheel. Official PyTorch 2.8 has no Intel Mac wheel. Do not silently
  downgrade dependencies or promise Intel inference support. Runtime support and
  support for the rest of the desktop application are distinct.
- Linux/Windows x64 have official PyTorch 2.8 CPU wheels. Their `+cpu` version
  identities must be pinned and checked consistently with native result validation;
  default Linux PyPI dependency resolution can pull CUDA dependencies. Linux native
  wheels impose at least glibc 2.28; the whole desktop distribution may be stricter.
- Lock exact interpreter builds and dependency artifacts per target, including
  transitive native files. The flat prototype list includes Demucs and prerelease
  ONNX tooling; review its closure rather than copying it as a release manifest.
  `onnx-weekly` supplies `onnx`, while `onnxruntime` supplies a different module;
  their existing installed file inventories do not overlap.
- The current graph cannot simply become a wheel-only installer:
  [Demucs 4.0.1](https://pypi.org/project/demucs/4.0.1/#files) is source-only, while
  [diffq 0.2.4](https://pypi.org/project/diffq/0.2.4/#files) has no CPython 3.12
  wheels and uses a native extension in the current Mac environment. Source-only
  does not itself imply native compilation for every package, but strict binary
  installation rejects these missing wheels. Compiler-free setup needs a narrow
  graph change or installer-bundled wheels and actual per-target verification.
  Do not copy the development setup script unchanged into the shipped app.
- Bundled uv can acquire an isolated managed Python without using system Python;
  explicitly disable inherited configuration and control sources/cache/child paths.
  See [managed Python](https://docs.astral.sh/uv/concepts/python-versions/) and the
  [uv command reference](https://docs.astral.sh/uv/reference/cli/). Q11 accepts this
  bootstrap mechanism; implementation and platform verification are still pending.
- For Q17's post-success cache cleanup, use an explicit app-private uv cache
  and `--link-mode copy`; after owned setup processes exit, use `uv cache clean`
  rather than mutating cache internals. Keep managed Python outside that disposable
  cache because the installed environment depends on it. See
  [uv cache safety](https://docs.astral.sh/uv/concepts/cache/#cache-safety) and
  [link modes](https://docs.astral.sh/uv/reference/settings/#link-mode).
- Provide and explicitly resolve a pinned FFmpeg binary: the selected
  upstream Separator constructor executes `ffmpeg -version`, despite source decoding
  being native Rust. Q11 selects installer-bundled FFmpeg. Avoid unrelated
  adapter changes or aggressive dependency pruning until the environment is
  verified. FFprobe is not used by this path.
- Current runtime readiness checks exact key distribution metadata and FFmpeg
  presence, not native imports/execution. Relocation tests must remove reliance on
  workspace paths, host executable search paths and globally installed libraries.
- [Desktop CI run 38888391271](https://github.com/zantoku/OhMyLoop/actions/runs/38888391271)
  (2026-10-16, commit `a92b6ff02ba70c9b1325f7e9df06d62d6c7c4abb`): macOS/Ubuntu
  checks and unbundled builds passed; Windows failed compiling vendored Rubber Band
  C++ before tests/build. Root cause is not diagnosed here. No installed-app or
  packaged-inference proof follows from that run.

These findings are constraints, not successful platform verification evidence.
Preserve existing library scope: closed-app whole-folder
copy/move/reopen and manual backup instructions; no new migration, recovery or
journaling feature is implied by the original issue's wording.
