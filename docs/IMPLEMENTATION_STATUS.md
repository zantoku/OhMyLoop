# Measure mapping implementation and evidence

Updated: 2026-10-09. The first manual editor, practice integration and optional
automatic-draft workflow are implemented. Analyzer quality remains provisional;
this is not a release-readiness or broad music-accuracy claim.

## Delivered behavior

- A dedicated Measure map screen shares source audio and transport with practice,
  retains its own viewport, and supports wheel zoom, middle-button pan and minimap
  navigation. Mapping gestures edit timing annotations without replacing loops.
- Build a selected span by measure count or detected pulses, preview, then apply.
  Correct shared boundaries and fixed beat anchors by dragging, exact source
  coordinates or keyboard nudges. Split/merge, respace, meter/pulse/grouping changes,
  pickups, partial measures, free-time gaps, numbering origin, protection and
  session undo/redo are available. Protected conflicts leave the accepted map intact.
- The practice screen shows the accepted measure ruler, musical position and
  previous/next/Go-to-measure navigation. Existing section identities, source-based
  loops, playback speed and pitch remain independent of the map.
- Version-3 song documents hold version-1 maps. Valid version-2 documents migrate;
  invalid/future documents are preserved. Load and update validate exact decoded
  sample rate and frame count. A failed map save leaves the accepted map intact
  and keeps its preview available for retry.
- Optional Beat This! main-model analysis runs in a separate instance of the
  executable, with two inference threads and a one-hour input limit. Source PCM
  preparation runs away from playback; cancellation kills/reaps the inference
  process and discards its result. Candidates retain song/revision/model/processing
  provenance; raw activations live in a separate bounded cache. Applying suggestions
  requires a chosen meter/pulse/span and explicit preview/apply. Downbeat conflicts
  and stale map revisions are reported.
- Optional model installation downloads about 83 MB, checks pinned SHA-256 hashes,
  and retains MIT notices and provenance. Manual mapping works without models.
  Installed models support offline operation through `OHMYLOOP_MODEL_DIR`.

## Architecture decisions

`src/timing/` separates model, validation, transactional edits and projections.
Meter/pulse/grouping are explicit per measure; comparing adjacent measures derives
meter changes. Beat anchors and shared boundaries are authoritative source-frame
coordinates. Tempo and numbering are derived, with BPM expressed in the selected
pulse unit. Gaps continue numbering from the global origin; independent numbering
resets after individual gaps are not included.

Persistence remains in `project`, native transport/source ownership in `desktop`,
and inference/model setup in `analysis`. Frontend mapping history and async job
orchestration are separate modules; Svelte renders validated projections. Shared
input primitives and playback settings serve both screens. Action scope permits
practice and mapping commands to share bindings when their scopes do not overlap.
Mapping commands have no new default keys; existing configurations stay valid.

The CLI remains available without the optional analysis dependency. Desktop enables
`beat-analysis`. Rust 1.89 is the declared minimum. Models are external assets;
`config/beat-model.json` and `licenses/beat-this-MIT.txt` record attribution and pins.

## Automated and native evidence

On this macOS host:

- **40 Rust tests pass**, including all existing DSP/callback tests, 13 mapping
  tests, source-identity/migration/preservation checks and analysis preparation/job
  error checks. Generated impulse locations at 8/22.05/44.1/48/96 kHz remain within
  one 22.05 kHz sample, with exact output duration. Preparation corrects Rubato's
  polyphase origin rather than trimming its advertised pipeline delay.
- **59 frontend tests pass**; Svelte/TypeScript reports zero errors or warnings.
  Coverage includes gestures, scoped bindings, preview/history, failed-save retry,
  screen changes and stale/disposed asynchronous replies.
- Formatting, workspace/all-target Clippy, vendor integrity and diff whitespace
  checks pass. The macOS app-only bundle builds at approximately 25 MiB.
- A separate QA app with workspace-only practice data exercised manual build/apply,
  shared-boundary dragging, exact 2.125-second edits, 10 ms/1 ms nudges, protection,
  real native inference, conflict warnings, detected-draft apply, pickup notation,
  practice ruler seeking, undo across screen switches and restoration after relaunch.
  Playback remained paused at the same source position during map edits. The mapper
  was inspected at 1180 × 810 and the minimum 820 × 620; its panes scroll while
  transport stays visible. The existing user app session was left open.

Reproduce the routine checks (omit `--offline` unless dependencies are cached):

```sh
export CARGO_HOME="$PWD/.cargo-home"
cargo fmt --check
cargo clippy --offline --locked --workspace --all-targets -- -D warnings
cargo test --offline --locked --release -p ohmyloop --features beat-analysis
npm run check
npm test
npm run desktop:build -- --bundles app # macOS
python3 scripts/check_vendor.py
git diff --check
```

Run native Cargo/Tauri builds and tests sequentially.

## Real model evaluation

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

## Remaining validation and scope

Linux/Windows CI jobs are configured, including analysis-feature tests and native
builds; they have not been run here. Only macOS has local build/UI/device evidence.
Physical middle-button dragging remains pending; automated gesture coverage passes.
Real guitar-song evaluation, small/main-model comparison and Python preprocessing/
prediction parity remain outstanding. Long-song resource use and broad device/load
coverage need further measurement. Code signing, installer distribution and the full
binary-release license inventory are release work.

Count-ins, inter-loop rests, ladders, section automation, sequences, export UI,
recording, stems and synchronized tabs remain deferred. No commit or push was made
while continuing this implementation.

## Download and analysis progress

Download jobs now report received bytes and the server's content length when
available. Preparation and spectrogram stages report processed frames; inference
reports completed segments through a decorator of the upstream public `Model`
interface. No prediction algorithm or third-party source was modified. Segment
counts follow pinned beat-this 1.1.0's 50 Hz mel grid and 1488-frame stride; tests
exercise the actual public pipeline across short, exact-edge and longer inputs.

The worker streams bounded newline-delimited progress updates through its stdout
pipe. A separate reader updates only the current running job; cancellation and
late replies cannot revive progress. UI polling shows elapsed time and percentages
for each stage, with indeterminate activity during model loading, verification or
unknown-length downloads. These are stage percentages, not an overall ETA.
The installed model folder (or intended download folder) appears below the model
setup button, wraps without truncation, and can be selected/copied.

A separate macOS QA instance performed a fresh actual model download: at 22 seconds
the UI displayed 53.6 / 83.2 MB and 64%, then restored Verify / reinstall with the
installed folder visible. A generated 330-second file showed preparation at 15%,
then 11 / 12 inference segments and 91% at 10 seconds, and completed successfully
with 661 suggestions (10.6 seconds reported analysis time). The existing user app
was not closed, and test models/practice data stayed in ignored workspace folders.

Development builds now optimize RTen's computation crates, Beat This!, FFT and
analysis resampling dependencies while keeping application code debuggable. The
updated development CLI/worker probe took about 2.0 seconds of inference and
7.9–8.1 seconds total per 33–34-second fixture; predictions matched the previously
reported results. Cancellation discarded the result in 26 ms. This is measured
current performance, not a controlled comparison against the old debug build.
Restart `npm run desktop` to pick up native/profile changes; the first dependency
rebuild takes longer. Release builds are unchanged in optimization policy.

Additional tests cover progress framing/limits, actual inference segment counts,
job identity/cancellation and elapsed-time freezing, monotonic PCM preparation,
UI progress rendering/polling and the model folder display. Current checks are
40 passing Rust tests and 59 passing frontend tests, zero Svelte/TypeScript
warnings, clean workspace Clippy/format checks, and a rebuilt macOS app bundle.

## Beat-aligned first drafts and clear-map workflow

**Create measures** now offers explicit Detected beats and Even spacing (manual)
sources. A completed analysis selects Detected beats. The manual count input is
absent in that mode; Preview calculates full/partial measure counts in Rust and
places boundaries/anchors at observed pulse frames. The summary explicitly counts
the last pulse as an end boundary, so full/partial lengths count pulse intervals.
First-downbeat alignment snaps to the nearest detected pulse independently of
replacement span bounds. A changed draft requires another preview before Apply.

Draft form state lives in `MeasureBuilder.svelte`; selected-measure notation has
separate state and shared `MeterFields.svelte` presentation/parsing. Choosing a
measure does not overwrite draft settings. Repeated status replies for the same
job do not reset them. Inspector refresh compares view content as well as revision,
so replacing a preview at the same accepted-map revision updates its fields.
Model setup is labeled Analysis model; the detected-preview action is no longer
hidden inside that disclosure.

The Clear map action removes all accepted measures/anchors, including protected
ones, resets numbering and preserves the source/ID counter. Mapping Undo restores
the exact annotations, IDs and protection. It retains detected suggestions for
rebuilding, and practice sections/loops/controls stay independent. An unapplied
preview alone is discarded without a document write.

Native macOS QA on generated demo audio confirmed automatic source selection,
1.13-second alignment snapping to 1.00 seconds, six measures in 4/4 (five full and
one partial), then eight in 3/4 (seven full and one partial) from the same 23 pulse
marks. Apply became disabled after a settings change until a fresh preview. Clear
removed an accepted map containing a protected measure; Undo restored its eight
measures and protection while playback stayed paused at zero. QA used isolated
workspace practice data and existing test models; the user app was left open.

Current checks: 40 Rust tests, 59 frontend tests, zero Svelte/TypeScript diagnostics,
clean formatting/workspace Clippy and a rebuilt macOS app bundle. New domain tests
cover first-pulse snapping, irregular anchors, compound-meter count derivation and
explicit clear; UI tests cover source modes, stale previews, same-revision inspector
refresh, preserved screen state, protected-map undo and untouched practice settings.

## Authored-map listening and paged review

The mapping screen now has a metronome with click level and a Follow playhead
toggle. Clicks and the transport's musical position follow the displayed green
draft, reverting to the accepted map on discard. Leaving mapping disables clicks;
reentering preserves the session preference. Mapping review state is source-bound
and ephemeral, with no song-document schema change. Both toggles use centralized
button/action/help metadata and support configured keyboard bindings.
Both toggles default on for each new song session; clicks wait for an authored
map or draft so an empty map does not attenuate playback.

`timing/pulses.rs` derives pulses and measure/group accents from authored meter,
pickup offset, partial length and piecewise beat anchors. Detected candidates are
never a playback clock. `metronome.rs` converts the bounded schedule to device PCM
frames with the same rounding/truncated EOF as seeking/resampling, then mixes
fixed-pitch clicks after DSP on the render worker. It resets click history on seek
and handles fractional source progression at loop wraps. The callback remains
unchanged. Live schedule/level changes retain the existing ~50 ms queue latency
plus hardware latency. Click mixing reserves headroom; timing remains the engine's
nominal source estimate, not sample-accurate Rubber Band transient alignment.

Paging preserves viewport span, handles skipped pages and backwards seeks/loop
wraps, and clamps the last page to EOF. Following defaults on and runs only during
playback. Wheel zoom, pan/minimap navigation and Fit measure/Whole song preserve
following; only the explicit toggle changes it. Follow playhead resumes it at
the current position.
Explicit seeks reveal the resulting native playhead position in both screens,
even while paused or with following off, without changing zoom or the toggle.
This includes Left/Right, Enter and other seek actions; Enter also reveals zero
when the playhead was already there but the viewport was manually panned away.

Current verification: 46 Rust tests and 71 frontend tests pass, zero Svelte/
TypeScript diagnostics, clean workspace Clippy/format/diff checks, and a rebuilt
25.09 MiB macOS app bundle. Generated-output tests cover actual DSP click spacing
at 0.5/1/2 speed with transposition, source-rate conversion, rubato anchors, 6/8,
7/8 grouping, pickups/partials/gaps, exclusive endpoints, paused prebuffering,
seeking and fractional loop wraps. Controller tests cover source identity and
unsaved review-map state. UI tests cover draft/discard/volume/screen transitions,
audio-error recovery, coalesced late replies, paging and manual navigation.
The default/seek tweak adds keyboard navigation checks in both screens while
paused and playing, including manually suspended following and preserved zoom.
No Rust/audio changes were needed for this tweak; native/device evidence below
is from the metronome implementation, not a new hardware run.

The reproducible hardware probe on KT_USB_AUDIO at 48 kHz passed with 1,692
callbacks, zero underrun events/device errors, max render 1.37 ms and last seek
setup 1.12 ms. It exercised clicks, speed/pitch, seek, looping, a live draft change,
pause, click disable and restart. An earlier run during a native build recorded
one underrun; the repeat without that load passed. This does not establish broad
device/load robustness or subjective song/click alignment quality.

```sh
cargo build --locked --release --example map_review_probe
target/release/examples/map_review_probe
```

The probe audibly plays generated material and touches no user song/settings.
On Windows use the `.exe` path. Native visual QA for these new controls remains
pending: the isolated QA app launched successfully, but the server restarted
before inspection and desktop-control tools were unavailable afterward. The QA
process was cleaned up; the user's existing app was left untouched. Linux and
Windows have not been locally verified.

## Map-review layout and visual detail

Analysis, Analyze song, suggestion visibility and progress now occupy the sidebar;
model installation remains a secondary disclosure with its folder path preserved.
Create measures stays alongside analysis. Detected suggestions show before mapping
and hide by default for an authored map or green draft; an explicit checkbox choice
persists across screen switches. The new `map_show_detected` action shares the
central keymap/button/help metadata. Candidate data remains available to rebuild
drafts regardless of visibility.

The timeline uses a presentation-only density helper to show every 2nd, 4th, 10th
or larger-step beat marker. Fixed anchors and detected downbeats are thinned too.
Stable sequence indices avoid phase changes while panning; zoom restores detail.
Sorted-frame window lookup and a separately derived beat array avoid rebuilding
all marker descriptors on each pan. Neither source coordinates nor click schedules
are edited by this display rule.

Play/pause, source time and speed are in a compact waveform overlay, with Loop off
when needed. Overlay pointer/wheel input is excluded from timeline gestures, and
speed changes retain pitch. The large mapping speed/pitch strip is removed. Measure
notation/boundary/numbering/protection tools, anchors/splits and respace/free-time
operations now live below the waveform, spanning both columns. `MeasureTools.svelte`
owns their form state and parsing; the editor still owns validated edit sessions,
preview/apply, undo and persistence. The timeline accepts playback UI as a snippet
and has no action-execution or playback-engine dependency.

Current frontend verification: 81 tests pass, zero Svelte/TypeScript diagnostics,
clean diff whitespace, production frontend build and rebuilt macOS bundle. Tests
cover suggestion visibility without map mutation, density thresholds/phase/gaps,
restored detail, fixed-anchor/downbeat thinning, overlay gesture isolation,
speed/pitch independence and relocated boundary/anchor edits through persistence.
The prior 46 Rust tests/device evidence still applies to the unchanged native
engine; no new hardware run was needed for these presentation changes. Native
visual review is still pending because desktop-control tools are unavailable.
Linux/Windows builds have not been run locally.

Follow playhead now changes only through its explicit action. Fit/Whole song,
minimap navigation, middle-button panning and wheel zoom preserve the setting.
The mapping playback overlay uses a single speed percentage input. Updated
regressions cover these navigation paths, manual suspension/resumption, keyboard
seeks and numeric speed changes that preserve pitch. All 81 frontend tests,
Svelte/TypeScript checks and the production frontend build pass for this refinement;
the native bundle and hardware checks were not rerun.

Selecting a measure in the timeline now seeks through the existing playback
action to its displayed source-frame start. Draft starts are respected, and
playback state, speed/pitch and saved maps are preserved. Existing outside-loop
seek behavior applies. Timing-handle selection/dragging/nudging and automatic
selection remain editing operations without seeking. The 35 focused App/timeline
tests, Svelte/TypeScript checks and production frontend build pass; native code
is unchanged and no new device run was made.

Waveform seeking now selects the measure containing the exact clicked source
frame, including draft ranges. Playback selects the current displayed measure
without seeking, independently of viewport following; shared endpoints belong
to the next measure and unmapped audio has no playing selection. A shared
presentation lookup uses the ordered ranges projected by Rust.

Boundary movement now resets internal beat anchors in both attached measures,
giving uniform interpolation within their new durations and actual notated
extents. Rust owns this atomic rule for drag, nudge and numeric edits. Outer
edges/gaps affect only attached measures; unrelated anchors, identities, meter
and partial/pickup extents stay intact. Protection and invalid-range checks
still reject the edit. Same-frame moves retain anchors, and session Undo restores
both sides of a resize in one step. The inspector and boundary tooltip explain
the new default.

Verification for this refinement: 89 frontend tests, zero Svelte/TypeScript
diagnostics, 16 measure-mapping tests and 4 metronome tests (including actual
DSP click output), Rust formatting/Clippy, production frontend build and macOS
app bundle pass. Regression coverage includes exact-click selection, displayed
drafts, playback through pickups/gaps/backwards wraps without seek commands,
atomic resize/Undo, protected neighbors, meter changes and unaffected gap edges.
No new hardware/listening or native visual review was performed; Linux/Windows
builds have not been run locally.

Detected-measure creation now has **Beat timing tolerance ε (ms)**, initially
30 ms and editable from 0 through 1000 ms. The Rust fitting module keeps detected
measure endpoints exact and recursively splits at the greatest pulse timing
error only when it exceeds epsilon. Newly created segments are checked again,
so the resulting interpolation fits every detected pulse within tolerance.
Collinear pulses remain interpolated even at zero epsilon. This preserves the
visual anchor/interpolation distinction without storing a correction for every
detected beat. Manual anchors, raw analysis evidence and resize behavior are
unchanged by fitting settings; creating/applying a replacement draft remains
explicit and respects protected measures.

The draft summary reports retained anchor count and epsilon. Tolerance is source
time, independent of playback speed/pitch; it is session draft state and changing
it invalidates the preview for Apply. Invalid or empty input is reported without
editing the accepted map. Older build requests that omit epsilon use the native
30 ms default; saved-song/map schemas are unchanged.

Verification: 89 frontend tests, 20 measure-mapping tests and 4 metronome tests,
zero Svelte/TypeScript diagnostics, Rust formatting/Clippy, production frontend
build and macOS app bundle pass. New fitting regressions cover omitted defaults,
invalid epsilon, strict threshold equality, rechecking new segments, noisy
regular pulses, exact zero-tolerance fitting, 1/44.1/48 kHz source rates,
fractional/compound pulse units and partial tails. The UI test covers default,
numeric/empty/zero input, wire values, stale Apply prevention, summary rendering
and preserving settings across screen switches. No new hardware/listening or
native visual review was performed; Linux/Windows builds remain untested locally.

The selected Measure panel now has a compact Meter label with two numeric inputs
and a large slash, kind/partial extent, and one Apply changes action. The existing
map_notation action commits directly through the validated MappingSession and
preserves Undo; validation/save failures retain the accepted map and local form.
Its description now reflects the direct action, with stable keymap identities.
Advanced initially folds away grouping, pulse, propagation and exact boundary
positions. Pulse stays independent of the denominator, including dotted-quarter
clicks in 6/8. A lock/unlock icon beside the title runs map_protect and stays
available across panels; protected measures cannot apply notation.

Batch tools follows Create measures in the sidebar. Its cohesive BatchTools
component owns global first-number form state and the existing preview/apply
numbering workflow. Numbering availability depends on the map rather than the
selected measure. A visible TODO records future grouping operations across
matching time signatures; no new batch grouping execution is implemented.

Verification: 93 frontend tests, zero Svelte/TypeScript errors/warnings, clean
diff whitespace, production frontend build and macOS app bundle pass. Regressions
cover disclosure placement, number controls, batch-section ordering, validated
direct notation saves with Undo and failure/retry, and title locking across
panels. Native visual/interaction review used an isolated app copy and generated
demo: compact/expanded layouts, direct 4/4-to-3/4 save without a preview, protection,
Undo restoration and sidebar numbering preview/discard were observed. No new
hardware listening run or Linux/Windows build was performed. Native code is
unchanged.

Join and Split replaces Beat anchors and splits; numerical anchor position/offset
controls are behind Advanced, while dragging remains the primary timing edit.
Join derives a summed numerator over the common denominator, preserving actual
extents, source bounds and anchors, including a new anchor at the former shared
boundary. Protected or nonadjacent measures, mismatched denominators and a
combined numerator above the existing 99 limit reject the operation. The first
measure's pulse remains and explicit grouping is concatenated.

Split uses native-projected numbered pulse points, including interpolated and
fixed beats and pulses hidden by grouping/density. Rust derives source frame and
notated offset; both results keep the prior meter and become incomplete, with
anchors retained on either side. The user explicitly chose two partial 4/4 bars
for splitting 4/4 at beat 3; their summed-numerator join therefore yields a
partial 8/4. Fractional pulse units and pickup offsets are accounted for.

The range re-spacing panel is removed. The legacy native edit and action identity
remain for compatibility/tests, but the desktop action is unavailable. Reapply
Meter to rebuild even spacing within a measure. Free time has a selected-measure
removal action and a range picker with meter/pulse and Create measure from range.
Clicking uncounted waveform audio selects the range without changing the exact
seek time. Removal selects the newly uncounted range; reclaim creates one full
measure with fresh identities, selects it and returns to Measure. Rust forbids
overwriting any existing measure. All operations are direct, validated and
undoable; saved song/map versions and audio callback behavior are unchanged.

JoinSplitTools and FreeTimeTools own their form state and command parsing, while
MeasureTools retains panel/presentation ownership and MappingSession retains
persistence/history. Inactive panel markup is omitted while component state
stays mounted, fixing a native WebKit accessibility issue observed during review.

Verification: 99 frontend tests, 24 focused measure-mapping tests and 4 metronome
tests (including actual DSP clicks), zero Svelte/TypeScript diagnostics,
formatting/Clippy, clean diff whitespace, frontend production build and macOS app
bundle pass. Regressions cover fixed/interpolated numbered splits, grouped pulses
and pickups, summed-meter joins, protection/denominator/gap rejection, exact
timing/anchor identity retention, uncounted-range presentation, no-overwrite
reclaim, direct commits and Undo. Native review on isolated generated demo data
verified Join 4/4+4/4 to 8/4, Split at beat 3 to two partial 4/4 bars, Undo,
mark/reclaim/selection of the newly uncounted range, and Free time accessibility.
The review found and corrected a picker that initially retained a previous gap;
the regression now verifies selection before any waveform click. No new subjective
listening test or Linux/Windows build was performed.
