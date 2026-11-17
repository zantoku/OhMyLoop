# Issue #6 verification evidence

Measured on 2026-10-14, macOS arm64. This records development-runtime/backend
and isolated component evidence; production runtime packaging and installed-app
Linux/macOS/Windows proof remain issue #7. No recordings, weights, runtime,
generated outputs or browser profiles are committed.

## Real model and progress equivalence

Generated 44.1 kHz stereo inputs of 1 second (44,100 frames) and 12 seconds
(529,200 frames) were processed sequentially by the selected old direct adapter
and the new protocol worker with the same pinned Roformer-SW weights/config and
CPU/two-thread profile. Every sample in all six decoded float32 outputs was
exactly equal (`numpy.array_equal`). Short-input corrections matched exactly;
normal input declared none. Both yielded structured chunk counts 0, 1, 2 with
total 2. The original iterator order/DSP arithmetic were preserved.

| Generated input | Old adapter process time | New worker process time | New child peak RSS |
| --- | ---: | ---: | ---: |
| 1 second | 23.44 s | 19.26 s | 3,045,146,624 bytes |
| 12 seconds | 18.06 s | 17.48 s | 3,116,269,568 bytes |

These sequential times include process startup; worker-internal times were 18.987 s
and 17.174 s. They are measurements, not speed guarantees. Existing prototype
weights were test inputs for equivalence only; the app has no import/reuse pathway.
Local evidence: `/tmp/ohmyloop-issue6-model-equivalence/evidence.json` and adjacent
requests/results/stdout/stderr; reproducible driver
`/tmp/ohmyloop-issue6-model-equivalence.py` (temporary maintainer QA, not a fixture).

## Fresh installation and full-song native flow

The production `SeparationManager` and `Desktop` APIs were exercised through
`examples/separation_probe.rs`, using a newly downloaded app-owned package and
an isolated managed library at `/tmp/ohmyloop-issue6-desktop-qa`. The source was
read from the maintainer's existing managed recording and copied into the scratch
library; no user association or assets were changed. The probe reads persisted
notes/maps after completion without rewriting the expected state.

- Fresh install: 699,416,805 verified bytes, 152.389 seconds.
- Reference: 13,073,408 frames at 44.1 kHz (296.449 seconds), stereo, digest
  `72b4cad7e2e42653bc0efa6d86773745aaaa7dcab5744c9323152ff3086f9a39`.
- Python 3.12.13; audio-separator 0.41.1; torch/torchaudio 2.8.0; numpy 2.2.6;
  soundfile 0.13.1; librosa 0.11.0; explicit CPU, two intra-op/one inter-op threads.
- Full debug-build flow: 420.331 seconds; child load/inference/output work
  306.983 seconds; actual inference reported 38 chunks. Debug SHA verification
  and native preparation account for additional time.
- Child peak RSS: 5,374,410,752 bytes (about 5.01 GiB). This excludes the native
  app's memory; whole-song inference is not duration-independent memory usage.
- Sampled maximum logical new stem-directory bytes: 1,621,386,012. Final directory:
  1,424,383,270 bytes. Sampling can miss brief peaks; this is separate from the
  conservative admission estimate and excludes the existing recording/model.
- All six roles exceeded −60 dBFS. Completion retained all six and reported
  37/37 tasks, prepared playback available, no warnings, and playback paused at 0.
- Frozen seek and concurrent note-save checks passed. The existing accepted map
  was unchanged (None for this song); generated native regression tests cover an
  actual two-measure edit made during successful publication.
- `KT_USB_AUDIO` initialized with no audio error. This demonstrates device setup,
  not subjective listening or a new long-running callback/underrun result.
- A second real attempt cancelled at `worker.infer` showed Stopping and then
  Cancelled in about 0.1 seconds after the request (33.596 seconds total), kept
  the original attached set available and left transport paused.
  The previous association was verified unchanged by the probe. The cancelled
  attempt directory was removed; only the prior attached directory remained.

The scratch model was then removed through the production manager, and the
completed song reopened through Desktop: prepared stems remained available without
warning and playback stayed paused (`/tmp/ohmyloop-issue6-remove.log`).
The isolated QA library and generated outputs were removed after verification;
logs, measurements and screenshots remain in the temporary evidence paths below.
The user's library, prototype outputs and development runtime were left intact.

Local logs: `/tmp/ohmyloop-issue6-install.log`,
`/tmp/ohmyloop-issue6-fullsong.log`, `/tmp/ohmyloop-issue6-cancel.log`;
child resource/output identities and sampled disk data are retained in
`/tmp/ohmyloop-issue6-fullsong-monitor.json`. These local audio references are not
portable automated fixtures. No inference deadline or RAM admission was exercised.

The full-song generated outputs were also compared against the existing selected
prototype with bounded 65,536-frame reads: all six decoded float32 tracks were
bit-identical across all 13,073,408 frames, with maximum absolute difference 0.0.
WAV container hashes differ, so this is decoded-audio equivalence, not byte-identical
container evidence. Details: `/tmp/ohmyloop-issue6-fullsong-equivalence.json`.

## UI and automated verification

Isolated production-component fixtures exercised nine states at 1180×810 and
820×620 using a fresh disposable headless Chromium profile: missing model,
download, ready, inference, stopping, finishing save, completion, failure and
missing runtime. There were no runtime exceptions or document/horizontal overflow.
Screen/checklist scrolling remained usable; Cancel/Stopping/Finishing save/Go to
Practice remained visible. The review moved concise failure/diagnostic controls
above the model card. Actual native validation revealed technical paths in phase
headings; these were replaced by semantic phases before final review. This is
component visual evidence, not a full graphical end-to-end separation run.
Screenshots/metrics: `/tmp/ohmyloop-issue6-visual`. Disposable fixture files/server
were removed/stopped.

Generated/fake-worker regression tests cover bounded protocol and diagnostic
reads, duplicate fields/stale IDs/backwards progress, crash/hang/process-tree
cleanup, precise short correction, cooperative source/hash/decode/resample/write
cancellation, generated no-copy validation/cache reuse, native transport guards,
successful authored-map/note publication, saved gain/mute/position/control retention,
100 concurrent Cancel-versus-Committing races, checksum/size/partial failure,
verified-file retry and offline complete installation. Frontend tests cover explicit
Download → Ready → Separate, screen closure/job persistence, frozen controls with
notes available, Stopping until native completion, confirmed quit (including pending
install admission before native work is visible),
repeated quit while cleanup is pending, runtime-only checks without a quit prompt,
and stale polls.
Existing tests retain silence-boundary/headroom/cache/transport/gain coverage.

The workspace/all-target native suite passed 184 tests with 2 hardware checks
ignored during this pass. Three additional focused shutdown regressions passed:
50 races against worker-handle registration/admission, and waiting for publication
that already won cancellation, and two simultaneous callers waiting for delayed
cancellation cleanup or committing work. The frontend suite passed 217 tests;
Svelte check had zero errors/warnings. Formatting, strict workspace/all-target Clippy and production
frontend build passed. Independent Standards and Spec reviewers verified the fixes
and reported zero remaining findings, including the final install-admission and
concurrent-shutdown races. Default tests perform no model download and do not
require an inference toolkit.

## Progress refinement verification (2026-10-16)

The follow-up uses fixed measured-time weighting only for separation. Manual
attachment and playback-cache preparation retain task units. Step pies always
show their own actual byte/frame/chunk fraction. Unknown-duration stages hold
the overall value and show an indeterminate active row. Successful publication
confirmation owns the final 100%; error/cancellation finalize measurements while
leaving their workflow incomplete. Measurements are in-memory for the current
attempt and exposed only to the native maintainer probe, not persisted as user
history or shown as a new timing UI.

The shared checklist follows only its own viewport. Wheel/trackpad or keyboard
scrolling away pauses following; returning to the active row resumes it. Reopening
a running screen reveals active work, terminal lists stay inspectable, and a new
attempt resets following. Immediate scrolling avoids animation and respects
reduced motion. Determinate pie fill starts at 12 o'clock clockwise.

Independent headless Chromium review exercised the real production component at
1180×810 and 820×620 (the latter with reduced motion): real wheel away/pause/back/
resume, keyboard Home away/pause, opening/reopening, terminal retention, new-attempt
reset, unchanged focus/page scroll, and computed/screenshot pie geometry all passed.
No runtime errors or page/horizontal overflow were observed. Evidence and screenshots:
`/tmp/ohmyloop-progress-browser-review/`; repeatable local driver:
`/tmp/ohmyloop-progress-browser-review.mjs`. This is browser/component evidence,
not physical touch-device testing or an installed-app end-to-end UI run. No new
subjective listening claim is made.

### Full-song timing methodology

Two consecutive successful Bones runs use separate fresh scratch libraries,
reference copies, generated-output directories and playback caches. The installed
app model is reused read-only; model download is excluded and no user association,
recording, cache or model is changed. Each worker is a fresh Python process. OS
file caches are deliberately left warm (including a preceding generated-tone
smoke run); no global caches are purged and the machine is not CPU-isolated.
Routine development/test activity can run elsewhere on the machine, so these are
approximate development progress weights, not a controlled throughput benchmark.

Hardware: Apple M5 Pro, 18 logical CPUs, 48 GiB RAM; macOS 26.6.2 arm64.
Reference: stereo M4A, 13,073,408 source frames at 44.1 kHz (296.449 seconds).
Both input preparation and these playback caches use 44.1 kHz. The development
runtime is Python 3.12.13, audio-separator 0.41.1, torch/torchaudio 2.8.0, numpy
2.2.6, soundfile 0.13.1 and librosa 0.11.0. Profile `roformer-sw-cpu2-v1` retains
two intra-op/one inter-op threads, CPU float32, seed 0 and the existing model/DSP
settings. The model weights/configuration are freshly verified for every job.

The exact native build command is:

```sh
CARGO_HOME="$PWD/.cargo-home" cargo build --locked --offline --features stem-separation --example separation_probe
```

This uses Cargo's unoptimized development profile with debug info and the existing
per-dependency overrides in Cargo.toml (including optimized hound/rubato). It is
the runnable development path, not a production release/runtime benchmark (#7).
Model inference uses the unchanged pinned Python/PyTorch implementation. These
weights are not claimed to be calibrated for unmeasured release builds, other
hardware, resampling rates or materially different song lengths.

To repeat with an existing installation, set `OHMYLOOP_PROBE_MODELS` to the parent
of `roformer-sw-v1`, and use a new empty scratch path for each run:

```sh
OHMYLOOP_PROBE_MODELS="$APP_MODELS_PARENT" target/debug/examples/separation_probe run "$REFERENCE_RECORDING" /tmp/ohmyloop-progress-qa-a
OHMYLOOP_PROBE_MODELS="$APP_MODELS_PARENT" target/debug/examples/separation_probe run "$REFERENCE_RECORDING" /tmp/ohmyloop-progress-qa-b
```

The external model option is accepted only by `run`; installation/removal modes
reject it. The probe prints finalized `stage timings` JSON even on failure.
Intervals use one native monotonic clock and the same start/end as the attempt's
terminal elapsed time; their sum is the job total. The total starts at admitted
job creation, including thread startup, and ends after publication/normal cleanup
and releasing the playback/heavy-work guards. It excludes opening/importing the
scratch song before admission, initial runtime probing and model download.

Transitions are captured before visual throttling: native preparation emits its
stage before work; the child emits startup/input before heavy imports and input
reading/hash, loading before model setup, inference before demix allocation, and
writing before the first output/check/hash. Worker finish covers shutdown/reaping
before native result validation. The interval ends at the next native boundary or
received worker boundary, so small IPC/scheduling overhead belongs to the adjacent
stage. Validation and playback/cache/publication use their real native transitions;
short omitted/resampling work is still represented at its actual duration. No
interval is counted twice or synthesized from polling samples.

A preceding exploratory run using changed validation traversal reached the existing
bank-order safety check and failed; it is excluded from calibration. Validation
traversal/cache order remain unchanged (alphabetical), and the separation checklist
now matches that order. Before the successful full-song measurements, a generated
one-second real-model smoke test completed input correction, validation, publication
and cache reuse in 41.261 seconds. The six-role generated-audio regression also
verifies actual active-row order, publication/cache behavior and absence of reopened
completed rows. The machine's audio device was unavailable in this pass; successful
separation/cache preparation does not imply hardware playback or subjective audio
verification.

### Successful full-song results and fixed weights

Run A and B below are successful fresh attempts (local logs `run2` and `run3`);
the earlier failed exploratory `run1` is excluded. All six tracks were retained,
publication completed and notes/map/paused-position checks passed. Stage sums
agree with terminal job totals (within floating-point precision).

| Native stage | Run A (s) | Run B (s) | Fixed overall weight |
|---|---:|---:|---:|
| Admitted job setup/thread startup | <0.001 | <0.001 | Included in model verification |
| `model.verify` | 20.950 | 21.510 | 5.550% |
| `source.hash` | 0.145 | 0.146 | 0.038% |
| `source.decode` | 2.705 | 2.741 | 0.712% |
| `source.check` | 0.144 | 0.146 | 0.038% |
| `source.resample` | <0.001 | <0.001 | <0.001% |
| `source.write` | 0.797 | 0.870 | 0.218% |
| `source.pcm` | 3.164 | 3.190 | 0.830% |
| `worker.start` | 0.036 | 0.034 | 0.009% |
| `worker.input` | 1.107 | 1.617 | 0.356% |
| `worker.load` | 0.558 | 0.609 | 0.152% |
| `worker.infer` | 305.688 | 308.680 | 80.307% |
| `worker.write` | 0.733 | 0.755 | 0.195% |
| `worker.finish` | 0.283 | 0.273 | 0.073% |
| `result.validate` | 0.003 | 0.003 | <0.001% |
| `validation.reference.hash` | 0.152 | 0.151 | 0.040% |
| `validation.reference.decode` | 3.232 | 3.241 | 0.846% |
| `validation.reference.resample` | <0.001 | <0.001 | <0.001% |
| `validation.prepared.hash` | 3.279 | 3.274 | 0.857% |
| `validation.prepared.decode` | 1.783 | 1.784 | 0.466% |
| `validation.bass.hash` | 3.269 | 3.256 | 0.853% |
| `validation.bass.decode` | 1.777 | 1.779 | 0.465% |
| `validation.bass.resample` | <0.001 | <0.001 | <0.001% |
| `validation.drums.hash` | 3.257 | 3.259 | 0.852% |
| `validation.drums.decode` | 1.756 | 1.787 | 0.463% |
| `validation.drums.resample` | <0.001 | <0.001 | <0.001% |
| `validation.guitar.hash` | 3.262 | 3.242 | 0.850% |
| `validation.guitar.decode` | 1.765 | 1.768 | 0.462% |
| `validation.guitar.resample` | <0.001 | <0.001 | <0.001% |
| `validation.other.hash` | 3.246 | 3.248 | 0.849% |
| `validation.other.decode` | 1.775 | 1.756 | 0.462% |
| `validation.other.resample` | <0.001 | <0.001 | <0.001% |
| `validation.piano.hash` | 3.241 | 3.234 | 0.846% |
| `validation.piano.decode` | 1.776 | 1.762 | 0.463% |
| `validation.piano.resample` | <0.001 | <0.001 | <0.001% |
| `validation.vocals.hash` | 3.247 | 3.237 | 0.848% |
| `validation.vocals.decode` | 1.764 | 1.782 | 0.464% |
| `validation.vocals.resample` | <0.001 | <0.001 | <0.001% |
| `validation.headroom` | 5.486 | 5.479 | 1.433% |
| `cache.adopt` | <0.001 | <0.001 | <0.001% |
| `cache.save` | 0.008 | 0.007 | 0.002% |
| `attachment.publish` | 0.006 | 0.006 | 0.002% |
| **Total** | **380.395** | **384.627** | **100%** |

Each stage's arithmetic mean wall time is its fixed weight. The small initial
job-start interval is merged into model verification's weight, so all admitted
job time contributes once. `WorkflowTracker` normalizes the profile to 10,000
fixed units when creating an attempt; within-stage fractions use real units.
Values below one millisecond are shown as `<0.001`, not claimed to be zero.
Skipped or reused work hands off immediately while retaining the same denominator.
The native `finish()` confirmation alone completes the final fraction.

The profile is centralized in `src/stems/prepare/separation_weights.rs`; no user
history, ETA or elapsed-time-driven creeping is used. Inference now receives
80.3% of the overall bar, rather than one checklist row's equal share.
Differences between these warm-cache runs and other machines/builds are expected.
Local timings: `/tmp/ohmyloop-progress-run2.log` and
`/tmp/ohmyloop-progress-run3.log`; compact extracted report:
`/tmp/ohmyloop-progress-calibration.json`.


### Final checks and cleanup

The final workspace/all-target native suite passed **194 tests**, with **2 hardware
checks ignored**. The frontend suite passed **221 tests in 26 files**. Formatting,
strict Clippy, Svelte checking (zero errors/warnings) and production frontend build
passed. Exact final commands:

```sh
CARGO_HOME="$PWD/.cargo-home" cargo fmt --all -- --check
CARGO_HOME="$PWD/.cargo-home" cargo test --locked --offline --workspace --all-targets
CARGO_HOME="$PWD/.cargo-home" cargo clippy --locked --offline --workspace --all-targets -- -D warnings
npm test
npm run check
npm run build
git diff --check
```

Focused coverage includes weighted measured/unknown progress, fixed denominators,
publication-only completion, actual six-role traversal, never reopening completed
rows, skipped/reused work, failure/cancel/new attempts and stale terminal callbacks.
Follow coverage includes visible-row stability, manual pause/resume, reopening/new
attempt reset and fractional CSS-pixel rounding. Independent Standards and Spec
reviewers reported **zero remaining findings**; the Spec reviewer also independently
checked both raw successful timing traces against every mean profile constant.

Local final logs are `/tmp/ohmyloop-progress-native-final.log`,
`/tmp/ohmyloop-progress-frontend-final.log`, `/tmp/ohmyloop-progress-clippy-final.log`,
`/tmp/ohmyloop-progress-svelte-final.log` and `/tmp/ohmyloop-progress-build-final.log`.
Linux/Windows and production runtime/build timing were not exercised here.

Only the four freshly created scratch roots for the smoke/exploratory/two successful
runs were removed, after rejecting symlinks and verifying each exact owned root.
Small records, timing traces, extracted calibration JSON and browser evidence remain
under `/tmp/ohmyloop-progress-*`; generated copyrighted audio was not committed.
The cleanup report is `/tmp/ohmyloop-progress-cleanup.json`. The user's library,
app-installed package, prototype results and workspace Python runtime remain intact.
