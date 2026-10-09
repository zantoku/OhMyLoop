# Measure mapping implementation checkpoint

Paused at the user's request: 2026-10-09.
This is an implementation in progress, not a completed or released milestone.
Resume against [MEASURE_MAPPING.md](MEASURE_MAPPING.md).

## Current code

- Rust musical-time domain in `src/timing.rs`: integer source frames, rational
  quarter-note values, shared boundaries, changing meter/pulse/grouping,
  pickups and incomplete measures, free-time gaps, beat anchors, local edits,
  split/merge, spacing, protection, derived numbering and coordinate lookup.
- Song document version 3 adds the map. Valid v2 files migrate on load without
  changing practice settings; invalid/future files retain their preservation
  behavior. Actual decoded rate/frame count is checked on updates.
- A dedicated `MeasureEditor.svelte` screen is connected to the app shell via
  Measure map / Back to practice actions. It has its own timeline, inspector,
  preview/apply, history, and edit selection. Shared timeline helpers and
  playback settings preserve the practice workflow. Practice measure ruler,
  musical position and measure navigation are wired.
- Mapping and analysis orchestration are separate frontend modules; action
  availability includes practice/mapping scope. New actions have no default
  keys yet, preserving existing configuration files.
- Optional `beat-analysis` Cargo feature uses pinned `beat-this` 1.1.0 / RTen.
  Desktop enables it; default CLI operation is preserved. The declared Rust
  minimum is now 1.89. Cargo dependencies are fetched into `.cargo-home`.
- Native analysis manager starts a separate instance of the app executable in
  `--analysis-worker` mode before Tauri initialization. Jobs prepare shared
  source PCM, restrict inference to two threads, support cancellation, and
  keep candidates/provenance/raw activations outside the song document.
- Optional model download and offline `OHMYLOOP_MODEL_DIR` path are wired.
  URLs/checksums/provenance are in `config/beat-model.json`. MIT notices are
  in `licenses/beat-this-MIT.txt` and configured as bundle resources.
- Downloaded main and mel ONNX artifacts are in ignored
  `artifacts/beat-models/`; their hashes match the pinned configuration.
  These are development artifacts, not committed or installed user models.

## Checkpoint evidence

The following passed on this macOS host after the final checkpoint changes:

```sh
CARGO_HOME="$PWD/.cargo-home" cargo fmt
CARGO_HOME="$PWD/.cargo-home" cargo test --offline --locked -p ohmyloop --release --features beat-analysis
npm run check
npm test
git diff --check
```

There are **30 passing Rust tests** (including all existing audio checks,
8 mapping tests and 2 analysis-PCM tests) and **49 passing frontend tests**.
Svelte/TypeScript reports zero errors and warnings. A workspace Cargo check
passed earlier in this turn; final all-target lint/native bundle checks remain
pending. No native app window was inspected or relaunched for this revision.

A generated impulse test exposed an incorrect use of the resampler's advertised
pipeline delay in analysis preparation. The analysis path now aligns the
Rubato 0.16 polyphase origin rather than trimming that advertised latency.
Impulse locations at 8/22.05/44.1/48/96 kHz are within one model-rate sample,
and output length is exact. This verifies preprocessing alignment on the
covered inputs, not model accuracy or perceptual beat placement.

No actual model inference benchmark, Python parity run, subjective map review,
analysis-under-playback hardware check, Linux/Windows run, or new commit has
been performed. The user's existing native app/settings were not intentionally
changed; no real-song measure map was applied.

## Resume work

1. Review the code against the agreed interaction/domain contract. Split the
   large timing module into cohesive model/validation/edit/projection files
   before treating it as the maintained architecture. Check grouped beat
   labels, pickup displays, split/merge preservation, and source identity races.
2. Run final workspace Clippy and native bundle checks sequentially; resolve
   any failures. Review bundle resources and model/dependency attribution.
   Update setup documentation for Rust 1.89 and the analysis feature.
3. Exercise the real worker on generated annotated audio using the downloaded
   models. Record detection errors, duration/memory, cancellation and source
   alignment; compare small/main models and Python only when the reference
   environment is available. Do not confuse parity with ground truth.
4. Use an isolated `OHMYLOOP_DATA_DIR` under `artifacts` and the workspace model
   directory for native UI verification. Inspect normal/minimum-window layout,
   empty map, manual build/apply, dragging/nudging, meter changes, partial bars,
   undo/redo across screen switches, restoration, and automatic suggestions.
   Resolve usability problems, including dense full-song marker rendering.
5. Test model installation/missing/corrupt/offline states and cancellation;
   verify inference alongside actual playback with no callback regression.
   Add focused coverage for outstanding async/native boundaries as needed.
6. Reconcile the implementation plan with delivered behavior, update README,
   DESKTOP and THIRD_PARTY with exact commands/evidence/limits, and report each
   platform's validation honestly. A screenshot or configured CI is not proof
   of passing hardware/platform checks.

Retain the uncommitted work and the ignored model artifacts for resumption.
The original count-in, rests, ladder, sequencing, export, recording, stems and
notation features remain deferred.
