# First measure-mapping implementation pass

Drafted: 2026-10-09. Status: first editor and optional analyzer implemented;
analysis quality remains provisional. Verification and remaining evaluation: [IMPLEMENTATION_STATUS.md](IMPLEMENTATION_STATUS.md).
Research: [BEAT_MAPPING_RESEARCH.md](BEAT_MAPPING_RESEARCH.md).

## Outcome

Create, correct, save, and navigate a measure map for the current song. Provide
an automatic draft, but make manual mapping usable without an installed model
or successful analysis. Support tempo variation, changing and odd meters,
pickups, incomplete measures, and passages without a defined meter in this pass.

The map describes the original recording. Editing it does not stretch the
audio, change playback speed, or move existing sections and temporary loops.

## A dedicated screen

Add a **Measure map** button to the practice screen's header, next to the song
actions. Enable it when a song is loaded. It opens a screen in the same window;
**Back to practice** returns to the loop editor. Both actions belong to the
shared action/keymap catalog, with descriptions and configurable bindings.

The app shell owns the loaded song, document, and transport. Switching screens
does not reload audio or change play/pause, speed, pitch, or loop settings.
Each screen retains its own viewport. Share transport controls and timeline
navigation primitives; keep mapping gestures separate from loop gestures.

The mapping screen has:

- A whole-song minimap with click/drag navigation, plus a detailed waveform
  with pointer-anchored wheel zoom and middle-button panning.
- A musical ruler displaying measure numbers and beat positions, alongside a
  time ruler. Strong measure boundaries, lighter beat lines, meter labels,
  and explicit unmapped/partial spans make the interpretation visible.
- A toolbar for **Clear map**, adding boundaries,
  undo/redo, and fitting the selected measure or whole song.
- A panel below the waveform for the selected measure/boundary: meter, pulse unit/grouping,
  full/pickup/incomplete status, exact position, and protection from reanalysis.
- A compact waveform overlay for play/pause, source position and speed, plus
  previous/next measure and **Go to measure**. Pitch controls remain on Practice.
- An **Analysis** sidebar with **Analyze song**, **Show detected beats**, progress
  and model setup, followed by **Create measures** draft controls.
- Clear analysis progress, cancellation, missing-model/error states, and a
  visible distinction between suggested and accepted annotations.

Select a measure in the measure lane to seek to its displayed start, including
in a draft preview. Playback keeps its current playing/paused state; automatic
selection during map updates does not seek. Clicking waveform space selects the
containing measure while seeking to the exact clicked time. Shared endpoints
belong to the following measure; clicks in free time do not change selection.
During playback, selection follows the displayed map at the current source
position, independently of the viewport-follow toggle. Gaps and the exclusive
map end have no playing measure selected. This automatic selection never seeks.
Click a boundary/beat to select it and
drag its handle to change its source position. A mapping selection selects
map elements, not an A–B loop. Seeking follows the existing outside-loop rule;
mapping controls must not silently replace the practice loop. Provide an
explicit Loop off control if an existing loop obstructs listening.

Keyboard selection/nudging, exact position inputs, and visible keyboard focus
must support the same edits as dragging. Announce validation failures. Route
commands through scoped action availability so loop-editing commands are not
accidentally dispatched from this screen. Existing keymap files remain valid;
extend the catalog with backward-compatible defaults and explicit contexts.

## From detected pulses to measures

Beat This! provides candidate beat and downbeat positions, not authoritative
written time signatures or measure numbers. Do not assume each detected beat
equals the denominator of the chosen signature. The user supplies:

1. Meter, initially **4/4** as an editable assumption.
2. The note value represented by a detected pulse, initially a quarter note.
3. The location of the first full measure's downbeat.
4. Beat timing tolerance epsilon in source milliseconds, initially **30 ms**.

For example, four quarter-note pulses form 4/4; two dotted-quarter pulses form
6/8. Detected downbeats can suggest measure starts, but contradictory or missing
events require review. Half/double-time corrections must change the musical
interpretation or rebuild the selected grid, not playback speed.

Build measures for a selected span with a preview. The user may instead set
two measure boundaries and specify how many measures fit between them; this
creates an editable evenly spaced draft. Both methods work locally and leave
accepted material outside the selected span unchanged.

Detected measure endpoints remain exact source-frame positions. Within each
measure, fit evenly spaced notated pulse positions between the endpoints. Only
add a detected timing anchor when interpolation misses a pulse by more than
epsilon. Recheck each resulting segment after adding an anchor, so the final
piecewise map stays within epsilon at every detected pulse. This also handles the
actual duration of a final partial measure. Retained anchors keep the detected
source positions; collinear pulses need no anchors even at zero epsilon.

**Beat timing tolerance ε (ms)** is an editable number field in Create measures
for detected drafts, validated from 0 through 1000 ms. Its initial 30 ms value is
a tunable starting point, not a claim about detector accuracy. Tolerance is in
original source time, independent of playback speed/pitch. It does not alter raw
detections or existing manual corrections, and is session draft state rather than
a saved-song schema change. Only applying a rebuilt draft changes the map.

Meter changes occur at explicit measure boundaries. The inspector applies a
new meter/pulse interpretation from the selected measure to the next existing
meter change or a selected end. Preview any resulting regrouping/renumbering;
changing the label alone must not pretend the existing grid now fits.

Use quarter-note units internally, with explicit rational note values. A
normal n/d measure contains n * 4/d quarter notes. Meter and pulse unit are
separate; optional grouping such as 2+2+3 in 7/8 describes accents/pulse groups
without changing total measure length. Do not require every grouping to be
equal or supported by the automatic detector.

## Pickups, incomplete measures, and gaps

- The first full measure is numbered **1** by default. A leading pickup is
  displayed as **Pickup**, with its actual notated duration and offset within
  the nominal meter. Measure numbers are derived labels, not identities.
- An incomplete measure has an explicit actual notated duration shorter than
  its nominal meter. Support it at the end or within a song, for example at
  an edited cut or a meter change before a normal bar would finish.
- Inserting a meter change inside a measure first creates a boundary and
  explicitly marks the preceding fragment incomplete. Never cut/rearrange
  the audio to make the notation fit.
- Audio before the first mapped span, trailing audio after the last boundary,
  and internal free-time passages can remain **unmapped/free time**. Do not
  manufacture measures through silence or force the final boundary to EOF.
- A fermata or rubato inside a counted measure can be represented by beat
  anchors with longer source intervals. A truly uncounted passage is a gap;
  the following mapped span gets an explicit musical reference.

## Editing contract

Include these operations in the first pass:

- Add, move, and delete beat anchors; edit their notated position when a
  detected pulse was missing or mistaken for a subdivision.
- Add/move a measure boundary, promote a beat to a boundary, and merge adjacent
  measures. Split/merge operations ask for the resulting meter/actual duration
  when it cannot be inferred; they must not produce invalid full bars.
- Respace a selected range between two fixed boundaries, or divide it into a
  specified number of measures. Preserve internal user-protected anchors or
  report a conflict; never silently overwrite them.
- Change meter, pulse interpretation/grouping, and pickup/incomplete duration.
- Mark a span as free time and establish a new mapped span after it.
- Protect/unprotect corrected ranges and undo/redo all mapping edits.

Moving a shared measure boundary changes the two adjacent source durations;
later boundaries remain fixed. Beat anchors outside the affected measures do
not move. By default, moving a boundary removes internal anchors in both attached
measures, so their beats interpolate evenly over their new source durations and
actual notated extents. This applies to dragging, keyboard nudging and exact
position inputs. Only the attached measure changes at outer edges or beside free
time. A move to the same frame preserves anchors. Protected neighbors and invalid
overlapping/zero-length measures reject the whole edit. One Undo restores the
boundary and both measures' original anchors. Do not introduce implicit ripple
editing in this pass.

For between-anchor navigation, interpolate linearly in notated quarter-note
position. Arbitrary anchor spacing represents tempo drift, abrupt changes,
accelerando, and ritardando as a piecewise map. Insert more anchors for finer
within-measure timing. A separate smooth tempo-ramp editor is deferred; no
single global BPM value defines this map. Display derived tempo with an
explicit note unit so compound meter cannot silently change the meaning.

Commit a drag as one undoable edit, not hundreds of pointer updates. Provide
an immediate working preview, validate in Rust, and save only the accepted
valid result. Rejected commands leave the last valid map intact. Undo/redo
history is scoped to mapping and survives screen switching during the session.

## Domain and persistence boundaries

Implemented domain shape (validated in Rust before persistence):

| Entity | Responsibility |
| --- | --- |
| Analysis result | Immutable candidate beat/downbeat source frames, model/checksum/preprocessing provenance, and uncalibrated evidence scores |
| Shared boundary | Stable ID and integer source frame; adjacent measures reference the same boundary |
| Measure | Stable ID, boundary references, full/pickup/incomplete kind, actual notated extent, and optional pickup offset |
| Meter interpretation | Meter, pulse unit and optional grouping stored explicitly on each stable measure; changes are derived by comparison |
| Beat anchor | Stable ID, source frame and rational notated offset inside a measure |
| Map | Ordered mapped spans and free-time gaps, explicit numbering origin, protected ranges and revision |

Derive numbering, grids, source-to-musical and musical-to-source lookup, and
tempo summaries from the accepted map. Do not persist a second independently
editable tempo curve that can disagree with the anchors. Missing internal beat
anchors imply interpolation, not proof that the timing was verified.

Rust owns validation, map edits and coordinate conversion in a dedicated
musical-timing module. Persistence owns serialization/migration. The analysis
adapter owns inference jobs. Svelte renders map DTOs and dispatches commands;
timeline gesture helpers operate independently of domain/persistence logic.
Do not grow App.svelte into the editor, analyzer and musical-time model.

Validate stable IDs, strictly ordered source frames, source bounds, boundary
references, positive rational durations, meter/grouping consistency, pickup
offsets, and beat-anchor monotonicity in both coordinate systems. Source frames
remain tied to the decoded source sample rate regardless of playback speed.
Use half-open source intervals for measures and gaps: a shared boundary belongs
to the following measure/span. The last end boundary remains a valid endpoint,
including EOF, without creating a phantom next measure. Displayed beat numbers
are local to the chosen pulse interpretation; conversion uses quarter-note
positions rather than assuming every displayed beat is one quarter note.

Extend the current version-2 song document with a versioned map using an
explicit migration to the next document version. Preserve sections, controls,
loops and position byte-for-byte in meaning; migrate valid old documents and
retain the existing malformed/future-version preservation behavior. Define
limits on map size at the persistence boundary. Store large inference arrays
in a separate versioned cache, not in the document or status polling payload.

Manual edits and analyzer evidence remain separate. An analysis job is bound
to song identity, map revision, preprocessing and model identity. Changing the
song or editing the map while analysis runs makes its result a stale candidate,
not an automatic mutation. Reruns require an explicit preview/apply operation;
protected ranges and accepted edits outside the target span survive.

## Automatic analysis scope

Evaluate the pinned `beat-this` Rust port first, with RTen CPU inference and
the upstream main-model conversion. Use the small model for comparison during
evaluation; do not add multiple analysis settings to the product prematurely.
Validate port output and preprocessing before choosing the shipped artifact.
Rhythm Map remains a comparison/reference, not an initial required dependency.

Feed existing decoded audio through a source-aligned mono analysis path; account
for resampler delay/tail and validate timestamps. Run a single bounded job away
from the callback and playback worker. Limit inference parallelism, expose
stage-level progress and cancellation, and discard results after cancellation.
Do not promise an exact completion percentage when the backend cannot supply it.

Ship a clear optional model-install workflow with explicit download size,
version/checksum validation, license notices and an offline installed-model
path. Never require downloading the model to edit a map. Audit converted
weights separately from runtime/dependency licenses. Record the Rust port's
minimum compiler requirement and any dependency-version changes explicitly.

Analysis must not overwrite the map. Present candidate beats/downbeats, then
let the user choose interpretation and apply a draft to an empty/unprotected
span. If raw suggestions imply contradictory bar lengths, show the discrepancy
instead of assigning a confident time signature. Treat evidence scores as
relative model output, not calibrated map-correctness probabilities.

## Practice-screen integration

Once a map exists, show a measure/beat ruler and musical playhead position in
the practice screen alongside source time. Provide previous/next measure and
Go to measure through shared map lookup and action metadata. Unmapped audio
retains time navigation and displays no invented musical position.

Existing loop and section storage remains source-based. Seeking a measure uses
its exact boundary frame converted at the existing transport boundary; account
for the current transport's estimated audible-position limits. Whole-measure
loop selection, snapping loops to beats, and musical section-range inputs are
follow-ups rather than prerequisites for the editor.

## Work packages and review points

1. **Timing domain and persistence.** Define invariant-tested map types,
   coordinate lookup, transactional edits, v2 migration and generated timing
   fixtures. Resolve split/merge and pickup semantics before coupling views.
2. **Manual mapping screen.** Extract reusable timeline/navigation/transport
   pieces; implement entry/return, measure construction, inspector, local
   correction, undo/redo, protected ranges, and saved restoration. Deliver a
   usable editor on generated and real songs before adding inference.
3. **Analysis integration.** Evaluate the pinned port/model on representative
   material, validate PCM parity/resource use, add model setup and cancellable
   analysis, and connect candidate preview/apply to the manual workflow.
4. **Measure navigation and verification.** Add the practice-screen ruler and
   navigation, exercise the complete correction/restoration workflow, and
   run platform builds and targeted device checks.

These are slices of one first implementation pass, not claims of completion.
If the analyzer misses its quality/resource gate, keep the manual editor usable
and report the gap instead of presenting an unsupported automated map.

## Acceptance evidence

- Generated domain fixtures cover 4/4, 3/4, 6/8 with dotted-quarter pulses,
  additive 7/8, meter changes, tempo jumps/ramps, within-bar rubato, pickups,
  internal/trailing partial bars, silence and free-time gaps. Verify exact
  frame anchors and round-trip lookups; do not use MIR's 70 ms tolerance as
  the domain acceptance criterion.
- Edit tests show local boundary movement, regrouping, split/merge and
  undo/redo cannot corrupt later measures or existing practice settings.
  Persistence tests cover migration, reload, invalid data and stable IDs.
- Component tests cover scoped actions, waveform gestures, inspector
  validation, partial-measure displays, candidate/protected states, stale
  results, cancellation and switching screens without transport resets.
- Analysis evaluation distinguishes comparison with Python predictions from
  independent annotated truth. Record timing errors, missing/extra events,
  downbeat errors and bars requiring manual correction on guitar practice
  material; compare small/main models without claiming broad accuracy from
  one song or one aggregate metric.
- Measure inference time, peak memory and playback underruns with analysis
  active; record cancellation behavior and model-missing/corrupt/offline
  paths. Keep copyrighted evaluation music outside Git.
- Run formatting, linting, focused tests, native builds and actual-device
  smoke checks appropriate to the changes. Report Linux/macOS/Windows
  validation separately; configured CI is not evidence of passing runs.

## Deferred work

Count-ins, inter-loop rests, speed
ladders, per-section playback automation, sequences, export UI, recording,
stem separation and synchronized tabs remain separate milestones. The map
provides their future timing foundation; this pass does not implement them.
Also defer score import/alignment, polymeter, overlapping musical clocks,
manual tempo-curve drawing, ripple edits and automatic constrained neural
reanalysis. Preview/apply plus protected manual corrections is the first
robust reanalysis contract.

## Current draft workflow

The inspector's **Create measures** section selects **Detected beats** after an
analysis result arrives. Time signature, pulse note value and the first downbeat
are draft settings; the selected measure's notation has separate controls. The
first downbeat snaps to the nearest detected pulse within the replacement span.
Keeping alignment separate from span bounds lets a new draft replace an older
whole span even when its first detected measure begins later.

**Preview detected measures** derives count and reports pulses per measure,
full bars, final partial extent, actual first boundary and retained timing-anchor
count from Rust. The manual count field appears only for **Even spacing (manual)**,
which never uses detected
pulses. A changed draft requires a new preview before Apply. Input settings survive
screen switching and repeated status replies for the same analysis job.

**Clear map** is an explicit, undoable operation that removes all accepted measure
and beat annotations, including protected ones, and resets numbering to 1. It
preserves source identity, monotonic IDs and detected evidence for rebuilding.
Practice sections, loops, speed/pitch and play/pause state are independent. If
only an unapplied preview exists, Clear map discards it without a save.

## Listening to the displayed map

The mapping screen provides **Metronome on/off**, **Click level**, and **Follow
playhead on/off**. Metronome and following both default on for each song session.
Clicks begin once an authored map or draft exists. The metronome uses the green
draft while one is displayed, otherwise the accepted map. Applying retains that
timing; discarding returns to
the accepted map. Raw detected suggestions never drive clicks directly. Measure
starts receive a distinct accent, grouping starts a secondary accent, and the
chosen pulse note value determines the remaining clicks. Pickups keep their
notated offset; incomplete measures end early and free-time gaps are silent.
Arbitrary beat anchors change interpolation without becoming extra pulses.

Rust builds the bounded pulse schedule from original source frames, converts it
to the playback device rate, and mixes short fixed-pitch clicks after Rubber Band
on the render worker. Speed changes alter spacing while pitch changes leave the
click sound alone. Seeking resets click history; loop wraps restart the schedule
at the rounded source loop start. Pause stops both song and clicks. Live map/level
changes take effect after the existing bounded audio queue (about 50 ms, plus
device latency); no frontend timer schedules audio and the callback is unchanged.
Mixing reserves volume-dependent headroom for clicks to avoid clipping.

The musical footer follows the displayed map too. Review map, click enable/level,
and following are session state; they do not change the saved song schema. Leaving
mapping disables the metronome and restores accepted-map transport coordinates.
The click preference resumes on reentering mapping during the same song session.

Following defaults on and pages only during playback. Crossing the current
window's end advances by a full window, preserving zoom; backwards seeks and loop
wraps page backwards. The final page clamps to EOF. Manual panning, minimap
navigation, wheel zoom, Fit measure and Whole song preserve the follow setting.
Only the Follow playhead action changes it. With following on, playback keeps
revealing the playhead after navigation. Paused editing keeps the viewport still.
Explicit playhead navigation (including Left, Right and Enter) reveals the
resulting position in either screen even while paused or with following off.
It preserves zoom and the follow toggle, and does not move the view if the
resulting position is already visible. Rewinding also works when the playhead
is already at zero but the view has been panned elsewhere.

Clicks share the engine's nominal source-position estimate. Rubber Band's adaptive
timing can move individual transients around that estimate, particularly during
live speed changes; this is not a claim of sample-accurate click/transient alignment.
Count-ins, practice rests and exported clicks remain outside this review feature.

## Map review detail and editing layout

**Show detected beats** in Analysis controls only the raw suggestion overlay.
Suggestions show initially, then hide by default when an authored map or green
draft appears. An explicit checkbox choice wins for the rest of that song session,
including screen switches. Hiding suggestions does not delete them or change
draft construction, the accepted map, protection or the metronome.

Beat markers thin at discrete density levels: every 2nd, 4th or 10th, with larger
steps for very long songs. Fixed anchors and detected downbeats obey the same
visual density rule; they remain intact in the map and anchor selector. Sampling
keeps its phase while panning, and zooming restores detail. Measure boundaries
remain separate from these beat-detail levels. This is a presentation rule only.

The waveform overlay contains play/pause, source position and an exact speed
percentage input. A Loop off action appears when an existing loop is enabled.
Overlay input does not start timeline gestures or wheel zoom. Speed changes keep
the current pitch. Full pitch controls are available on the Practice screen.

Selected-measure tools occupy the full-width panel below the timeline, with
Measure, Beat anchors and splits, and Respace or free time panels. They retain
exact source-frame edits, preview/apply, protection and undo semantics. Analysis,
model setup and draft creation stay in the sidebar. Form state lives in the
cohesive `MeasureTools.svelte` component; map/session ownership stays in the editor,
and the timeline receives playback presentation through a Svelte snippet.
