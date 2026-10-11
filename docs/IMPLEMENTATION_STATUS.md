# Measure mapping implementation and evidence

Updated: 2026-10-11. Manual measure editing, practice navigation, map-review
listening and optional automatic drafts are implemented. Detector accuracy and
cross-platform release readiness remain provisional. This page describes the
current implementation; [implementation history](IMPLEMENTATION_HISTORY.md)
preserves earlier evidence and superseded behavior.

## Delivered workflow

The **Measure map** screen shares the loaded song and native transport with
Practice, retaining its own viewport. Wheel zoom, middle-button pan, minimap
navigation and precise seeking work in both screens. Mapping gestures edit
source annotations independently of saved sections and temporary loops.

**Analysis** holds Analyze song, Show detected beats, progress/cancellation and
optional model setup. Raw suggestions hide by default when an authored map or
green draft appears; an explicit visibility choice lasts for the song session.
Beat-marker density changes presentation only. The compact waveform overlay
contains play/pause, source position and one speed percentage input; pitch is
adjusted on Practice. Loop off is available when a practice loop is enabled.

**Create measures** has two sources. Detected beats becomes selected after
analysis and derives the measure count from the chosen meter, detected pulse
note value and first downbeat. Boundaries follow observed pulses; internal
anchors are retained only where interpolation exceeds the selected epsilon
(default 30 ms, range 0–1000 ms). Even spacing (manual) uses an explicit count
and ignores detections. Both preview a green draft before Apply map. Replacement
must contain whole unprotected measures; material outside the span remains.
Changing detected-draft settings requires a fresh preview.

The full-width editing panel has three tabs:

- **Measure:** compact Meter numerator/denominator, kind and partial duration,
  one direct Apply changes action, and a title lock. Advanced contains pulse,
  grouping, propagation until the next meter change/gap, and exact boundaries.
  Reapplying notation clears internal timing corrections in affected measures.
- **Join / Split / Shift:** three horizontal tool groups with vertical separators
  (stacked at narrow widths). Join sums numerators over a common denominator;
  Split before beat shows only beat numbers and produces two incomplete measures
  retaining the original meter. Shift uses the selected pulse count, default two,
  moving the selected and later bar lines through authored musical timing.
  Right expands the preceding meter; left shortens preceding material and extends
  the last measure. Consumed measures are absorbed, endpoints stay fixed, and
  meters grow to avoid overfull notation. Advanced holds numeric anchor edits.
- **Free time:** remove the selected measure's map while keeping audio, or reclaim
  a selected uncounted range as one full, evenly spaced measure. Reclaim never
  overwrites existing measures and creates fresh identities.

Notation, boundaries, anchors, protection, Join/Split/Shift, free time and Clear
map commit directly after validation and save. Draft creation and global numbering
retain Preview/Apply/Discard. **Batch tools**, below Create measures, owns First
full measure number and a TODO for future grouping across matching signatures.
The desktop has no range re-spacing control; its legacy action is unavailable.

Boundary resizing evenly spaces beats in both attached measures; unrelated
measures remain fixed. Shifting instead preserves the authored timing curve and
moves anchors between surviving bars. Optional re-snapping aligns new internal
barlines to detection and replaces affected anchors using the shared Create measures
epsilon (30 ms initially); pulse-count conflicts reject the whole edit. Re-snapping
defaults on and can always be toggled. Restore saved detection when the editor opens,
so an app restart does not require rerunning analysis. Without evidence, turn the
option off to shift while preserving authored timing. Both reject affected protection and invalid
coordinates. Shifts also reject free-time crossings and unsupported meter/fraction
limits. Undo restores the exact musical definitions, including absorbed bars;
revision and generated-ID counters remain monotonic. History keeps up to 50 edits,
survives screen switching, and is not persisted across song sessions/restarts.
Clear map explicitly removes protected annotations too, resets numbering and
retains detected evidence and practice settings.

Measure-lane selection seeks to its displayed start. Waveform clicks select the
containing measure and seek to the exact clicked time; free-time clicks open that
range. Playback selects the currently playing measure without seeking. If a shift
absorbs the selected bar, selection moves to the survivor covering its former
start. Practice displays the accepted ruler, musical position and previous/next/
Go-to-measure navigation. Practice keeps its section lane but replaces the separate
measure lane/timecode ruler with one waveform ruler: measure numbers and vertical
barlines in mapped audio, timecodes in free time. Loops snap to whole measures by
default; handles and A/B use barlines. A checkbox/Shift gestures permit free
placement, and exact time fields and saved sections retain precise bounds.

**Metronome** defaults on with 50% level, shared between both screens for the app
session. Practice clicks use the accepted map; mapping clicks use the displayed
authored draft/map, never raw detections. **Follow playhead** defaults on in both screens, with independent session toggles;
practice pages with or without a map.
Grouping controls accents; pulse controls click spacing. Click level adjusts
volume, speed changes spacing, and pitch does not transpose clicks. Paging keeps
zoom and handles seeks/loop wraps. Manual viewport navigation never disables
following; only its toggle does. Explicit seeks reveal the playhead even when
paused or following is off. These review preferences are session state.

Detailed semantics and limits: [MEASURE_MAPPING.md](MEASURE_MAPPING.md).

## Architecture and persistence

`src/timing/` separates model, validation, transactional edits, detection fitting,
source/musical projection, pulse schedules and suffix shifting. Source frames,
shared boundaries and explicit anchors define a piecewise timing curve. Tempo,
numbering and grids are derived. Meter, pulse and optional grouping belong to each
measure; gaps continue global numbering without independent numbering resets.

`src/project.rs` owns version-3 song documents containing version-1 maps. Valid
version-2 documents migrate; malformed/future documents are preserved. Native load
and update validate decoded source rate/frame count. Sections, loops and controls
retain their own source coordinates. Failed saves leave the accepted map intact:
a draft remains available for retry, while direct edits retain their form values.

`src/desktop.rs` owns native transport/source state. `src/metronome.rs` mixes
fixed-pitch clicks after DSP on the render worker; the callback does no analysis,
map editing or UI work. Click timing shares the nominal source estimate and the
bounded queue (~50 ms plus hardware latency), not exact Rubber Band transient
alignment. `src/analysis/` owns model setup, bounded progress and cancellable
subprocess inference; `src/analysis_pcm.rs` prepares source-aligned mono PCM.

Frontend MappingSession owns save orchestration/history; the app owns one shared
MetronomeSession, avoiding competing native requests on screen transitions.
AnalysisSession owns analysis job state. MetronomeSession coalesces click-map
and level updates. MeasureBuilder, MeasureTools,
JoinSplitTools, ShiftTools, FreeTimeTools and BatchTools own cohesive forms and
command adapters. Svelte renders native projections; timeline helpers own gestures
and density. Action identities, button bindings, scopes and help remain in
[ui/actions.ts](../ui/actions.ts), with validated configuration in `keymap.ts`.
New mapping actions have no default keys; old keymaps remain valid.

The CLI remains usable without `beat-analysis`; desktop enables that feature.
Rust 1.89 is the declared minimum. Beat This! 1.1.0 uses RTen CPU inference in a
separate instance of the executable, capped at two inference threads and one hour
of input. No Python runtime is shipped. Candidates carry song/revision/model/
preprocessing provenance and uncalibrated scores; raw logits use a separate cache.
Stale revisions are warned about and suggestions never apply automatically.

Model setup downloads roughly 83 MB, verifies pinned SHA-256 hashes and retains
MIT notices/provenance. Models are optional external assets and work offline once
installed. Progress reports bytes, preparation/spectrogram frames, completed
inference segments and elapsed time; stage percentages are not an overall ETA.
Loading/verification and unknown-length downloads show indeterminate activity.
The installed/intended model folder appears beneath the setup button. Cancellation
kills/reaps inference and discards results; network cancellation can wait for a
stalled read's request timeout. Development dependency profiles optimize inference
while keeping application code debuggable.

## Current verification

On this macOS host, for the current code:

- **73 Rust tests pass** with the analysis feature in release mode: 6 library,
  1 analysis-job, 2 analysis-PCM, 10 audio-risk, 10 desktop-workflow, 1 keymap-file,
  24 measure-mapping, 15 suffix-shift and 4 metronome tests. No doc tests are defined.
  The 43 mapping/shift/metronome tests cover actual DSP clicks, variable tempo/meter,
  pickups/partials/gaps, epsilon fitting, split/join/reclaim, absorbed measures,
  exact source timing, protection and invalid edits.
- **125 frontend tests pass** in 13 files. Svelte/TypeScript reports zero errors
  and warnings. Coverage includes gestures, scoped bindings, defaults/paging,
  selection, density, direct saves and draft workflows, failure/retry, Undo,
  screen switching and stale/disposed async replies.
- Rust formatting, workspace/all-target Clippy with warnings denied, vendor
  integrity (79 files) and whitespace checks pass. The production frontend and
  macOS app-only bundle build; the latest bundle is approximately 25 MiB.
- Native visual/interaction review used an isolated app copy and generated demo
  data: direct notation/protection/Undo, batch numbering previews, Join and
  numbered Split, free-time removal/reclaim/selection, both shift directions and
  exact Undo, absorption of a two-pulse tail into 6/4, retained tempo-change timing,
  and the horizontal layout with a numeric-only split menu were observed. The
  latest practice QA confirmed the single ruler/barlines, whole-measure and free
  loop drags, shared metronome toggle across screens, and native playback with
  clicks enabled at the default 1180 × 810 window size. Practice following was
  also observed paging from 0–3 s to 3–6 s during native demo playback at 4× zoom;
  toggling following off retained the 0–3 s view through subsequent playback to EOF.
  Shift re-snapping was also exercised after real analysis of generated demo audio:
  the checkbox and shared 30 ms epsilon were visible, a right shift saved detected
  internal barlines with zero fixed beat anchors, and one Undo restored all measure
  definitions and boundaries exactly. A further native restart check restored
  those saved suggestions with no installed model, showed re-snapping checked by
  default, allowed toggling off and back on, and saved a re-snapped right shift
  without rerunning analysis.

The full Rust suite, workspace lint, frontend tests, type checking and macOS app
bundle build were rerun for the 2026-10-11 shift re-snapping refinement. Native
interaction evidence from earlier passes remains applicable to unchanged tools;
re-snapping is covered by generated-data native domain tests, frontend workflow
tests and an isolated native interaction pass. Original practice-window and
analyzer/model-installer QA evidence is retained in
[DESKTOP.md](DESKTOP.md) and [implementation history](IMPLEMENTATION_HISTORY.md).
There is no new subjective click/audio quality assessment in this iteration. Linux/Windows CI matrices
are configured; no passing run for this revision has been verified here.

Reproduce routine checks (use offline mode only if dependencies are cached):

```sh
export CARGO_HOME="$PWD/.cargo-home"
cargo fmt --all --check
cargo clippy --locked --workspace --all-targets -- -D warnings
cargo test --locked --release -p ohmyloop --features beat-analysis
npm run check
npm test
npm run desktop:build -- --bundles app # macOS
python3 scripts/check_vendor.py
git diff --check
```

Run native Cargo/Tauri builds and tests sequentially. The desktop build also runs
`npm run build`; frontend-only production checks can use that command directly.

## Recorded real-model evaluation

These measurements were recorded during analyzer integration and were not rerun
for this documentation audit.

`examples/analysis_probe.rs` generates original annotated percussion/harmonic audio
and runs the actual subprocess adapter. These recordings do not establish accuracy
on guitar practice material. One-to-one event matching uses independent generated
truth; this is not comparison with Python predictions.

| Generated case | Beats matched / truth / predicted (70 ms) | Downbeats matched / truth / predicted (70 ms) |
| --- | --- | --- |
| Steady 4/4, 33 s | 64 / 64 / 67 | 16 / 16 / 17 |
| Tempo changes, 4/4 → 3/4 → 4/4 and rubato, 34 s | 60 / 64 / 62 | 13 / 18 / 21 |

Matched beats were within 20 ms in these fixtures. The changing-meter result needs
manual correction, especially at downbeats; neither result establishes a reliable
whole-song measure interpretation.

During actual default-device playback (`KT_USB_AUDIO`), both inference cases
reported **zero underrun events and zero device errors**, with 876/723 callbacks.
Maximum playback render time was 3.05/3.10 ms. Inference took 3.72/3.35 s (4.67/3.86 s
including preparation/process overhead) while a native bundle build was also active.
A separate run without that load took roughly 1.8–2.1 s of inference. Cancellation
completed in 57 ms in the instrumented device run and discarded the result.
`/usr/bin/time -l` reported maximum resident set size of 529,039,360 bytes (~505 MiB)
for the probe/worker run; this is not the app's steady-state memory footprint.

A fresh official-source download through the actual model installer completed in
46.9 s, verified both pinned checksums, wrote license/provenance files, and then
supported inference. Missing/corrupt-model tests return errors without modifying
model input files or producing a map. Model-install cancellation checks between
chunks; a stalled network read can wait for its configured request timeout.

```sh
cargo build --locked --release --features beat-analysis --bin ohmyloop --example analysis_probe
# An already-installed model folder:
target/release/examples/analysis_probe target/release/ohmyloop artifacts/beat-models artifacts/analysis-probe --device
# Download into a fresh folder and verify the installer too:
target/release/examples/analysis_probe target/release/ohmyloop artifacts/installer-models artifacts/installer-probe --install
```

The example writes generated WAVs, `report.json` and caches into the selected output
folder. `--device` requires real sound hardware and audibly plays generated material.
On Windows use `.exe` paths. Model artifacts, reports, caches and QA data are ignored
by Git and can be regenerated.

## Recorded map-review hardware evidence

The generated `map_review_probe` run on KT_USB_AUDIO at 48 kHz recorded 1,692
callbacks, zero underrun events/device errors, max render 1.37 ms and last seek
setup 1.12 ms. It exercised clicks, speed/pitch, seek, looping, a live draft change,
pause, click disable and restart. An earlier run during a native build recorded
one underrun; the repeat without that load passed. This is earlier device evidence,
not a new check after editing/layout refinements, and it does not establish broad
device/load robustness or subjective song/click alignment quality.

```sh
cargo build --locked --release --example map_review_probe
target/release/examples/map_review_probe
```

This audibly plays generated material and touches no user song/settings. Use the
`.exe` path on Windows.

## Remaining validation and scope

Real guitar-song evaluation, small/main-model comparison and Python preprocessing/
prediction parity remain outstanding. Generated independent truth and the user's
positive detector feedback do not establish broad accuracy. Long-song resource use,
old x86 CPUs, physical middle-button dragging, broader device/load behavior and
click/transient alignment need further evaluation. Only macOS has local native
build/UI/device evidence. Minimum-window behavior was reviewed earlier; the latest
three-column layout has native review at the default window size, with narrow
stacking implemented but not rechecked natively at the minimum size.

Code signing, installers, automatic device reconnection, streaming large files and
a complete binary-release dependency-license inventory remain release/engineering
work. Whole-measure loop selection/snapping, per-gap numbering resets and batch
matching-meter grouping are not implemented. Count-ins, inter-loop rests, speed
ladders, section automation, sequences, export UI, recording, stems and synchronized
tabs remain later milestones in [PLAN.md](../PLAN.md).
