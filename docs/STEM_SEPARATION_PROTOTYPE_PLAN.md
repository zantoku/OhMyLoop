# Stem separation prototype implementation plan

Date: 2026-10-12. Status: isolated prototype implemented; local verification is
recorded in [the prototype documentation](../scripts/stem_prototype/README.md).
This is not a production feature. Question: can existing six-stem models produce
useful guitar study audio and backing tracks on an ordinary desktop?

This follows [the research](research/STEM_SEPARATION_RESEARCH.md). At the user's
request, licensing is deferred as a prototype selection gate. Retain upstream
notices and record artifact provenance; this does not establish distribution
permission or revise the research's findings.

## Deliverable and scope

Build a standalone command-line experiment in `scripts/stem_prototype/`, plus
an additive Rust example for source preparation. Keep current desktop, CLI,
practice documents, accepted measure maps and playback behavior intact.
Generated tools, virtual environments, checkpoints and audio live under the
already ignored `.tools/` and `artifacts/` directories.

The primary comparison is Demucs `htdemucs_6s` versus the six-stem
`BS-Roformer-SW.ckpt` used by feedBack. Use the existing separation libraries;
do not copy feedBack's application implementation. A later vocal comparator
can use Kimberley's MelBand model; it is not necessary for the first guitar
prototype. No remote audio service, neural-model training, permanent mixer UI,
native model conversion or production packaging is included.

## Implementation steps

1. **Prepare source audio.** An additive `stem_prepare` Rust example uses
   `Audio::load`/Symphonia and existing delay-compensated resampling. Write
   float stereo WAV and preparation metadata with full input SHA-256, original
   sample rate/frame count, selected source-frame span, prepared rate/frame
   count and preprocessing version. Default to a bounded excerpt; expose
   explicit start/duration and whole-song choice. Coordinates remain on the
   original recording. Never overwrite source files.
2. **Provide reproducible setup.** Document an isolated, pinned Python
   environment and exact setup/run commands. Separate dependency installation
   from model acquisition. Expose model-list/status and explicit download
   commands; keep every cache in the workspace. Use CPU as the correctness
   baseline; accelerator use must be explicit or reported with actual device.
3. **Separate in a child process.** A small parent CLI selects a model and
   launches a worker using direct Demucs and/or `audio-separator` APIs as
   appropriate. The worker receives only prepared PCM. Stream useful stage
   information and upstream progress, bound thread counts, and terminate/reap
   on cancellation. Do not put inference in the playback engine.
4. **Validate and retain results.** Record exact model/config hashes, package
   versions, inference settings, source/preprocessing identity, elapsed time,
   output geometry and peak resource data where available. Use isolated job
   directories and publish a completed manifest only after validation. Cache
   reuse requires matching identities and verified output hashes. A cancelled
   or failed job must never appear complete. Reject non-finite or misaligned
   outputs; any crop/pad/resampling correction must be explicit and recorded.
5. **Make the experiment audible.** Save lossless, unnormalized stem audio.
   Provide `mix` with per-stem gains and convenience guitar-only, no-guitar,
   reconstructed sum and original-minus-guitar outputs. Distinguish estimated
   accompaniment from subtraction residual. Apply a documented common gain
   when headroom is needed, rather than independently normalizing stems.
   Print paths that can be opened in the existing app for normal looping,
   speed and pitch controls. Derived files are separate songs for this pass;
   do not silently transfer annotations or claim multistem transport support.
6. **Verify and report.** Run focused generated-audio checks for frame/span
   conversion, resampling geometry, identity/cache invalidation, validation,
   mix math and cancellation/publication. A fake backend may exercise process
   lifecycle, but its output must never be labeled model inference. Attempt a
   short real model smoke on generated audio if setup/download works within
   available resources. Record setup or network failures honestly and leave
   exact commands for retry. Generated tones test plumbing, not guitar quality.

## Intended commands

The implementation README owns exact final syntax. Keep entry points simple:

```sh
python3 scripts/stem_prototype/cli.py --help
python3 scripts/stem_prototype/cli.py models
python3 scripts/stem_prototype/cli.py download --model demucs-6s
python3 scripts/stem_prototype/cli.py separate path/to/song.flac --model demucs-6s --start 30 --duration 20
python3 scripts/stem_prototype/cli.py separate path/to/song.flac --model roformer-sw --start 30 --duration 20
python3 scripts/stem_prototype/cli.py mix path/to/completed/manifest.json --preset no-guitar
```

Heavy operations run with the documented workspace Python environment. Help
and model descriptions should work without installing the inference stack.

## Acceptance and evidence

- A user can set up, download a selected model, separate a chosen local passage,
  and open guitar/backing WAVs in OhMyLoop using documented commands.
- Original recording and authored data remain untouched; failure/cancellation
  leaves no reusable completed result.
- Output manifests expose geometry, identities, versions, settings and measured
  duration; cache hits are explainable and reproducible.
- All stem mixes have a shared timeline, explicit gain policy and finite audio.
  Matching frame counts alone do not prove perceptual/sample alignment; record
  that limitation until real model transient tests establish it.
- Rust formatting/linting and focused prototype checks pass where runnable.
  Automated checks, real model smoke, hardware playback and subjective listening
  are reported separately. No Linux/Windows or listening pass is inferred.

## Next decision after the prototype

Evaluate clean/distorted guitar, overlapping lead/rhythm, sparse passages,
reverb and cymbal-heavy mixes on user-chosen recordings. Compare guitar clarity,
backing-track damage and artifacts after slow playback, alongside time, memory
and storage. Only those results justify choosing a model, adding a live mixer,
or investigating Rust/ONNX inference. Keep production integration separate.
