# Measure mapping: current behavior and implementation contract

Drafted: 2026-10-09. Updated: 2026-10-10. Status: manual editing, map-review
listening, practice navigation and optional analyzer implemented;
analysis quality remains provisional. Verification and remaining evaluation: [IMPLEMENTATION_STATUS.md](IMPLEMENTATION_STATUS.md).
Research: [BEAT_MAPPING_RESEARCH.md](BEAT_MAPPING_RESEARCH.md).

## Outcome

The editor creates, corrects, saves and navigates a measure map for the current
song. Automatic drafts are optional; manual mapping works without an installed
model or successful analysis. The map supports tempo variation, changing and odd
meters, pickups, incomplete measures and passages without a defined meter.

The map describes the original recording. Editing it does not stretch the
audio, change playback speed, or move existing sections and temporary loops.

## A dedicated screen

The **Measure map** button in the practice screen's header next to the song
actions is enabled when a song is loaded. It opens a screen in the same window;
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
- A toolbar for **Undo**, **Redo**, **Clear map**, **Fit measure** and
  **Whole song**. New internal boundaries are made by splitting a measure;
  new mapped spans are made through Create measures or Free time.
- A panel below the waveform for the selected measure/boundary: meter, pulse unit/grouping,
  full/pickup/incomplete status, exact position, and protection from reanalysis.
- A compact waveform overlay for play/pause, source position, speed percentage
  and Loop off when needed. Previous/next and Go-to-measure navigation sit in the
  footer. Pitch controls remain on Practice.
- An **Analysis** sidebar with **Analyze song**, **Show detected beats**, progress
  and model setup, followed by **Create measures** draft controls.
- Clear analysis progress, cancellation, missing-model/error states, and a
  visible distinction between suggested and accepted annotations.

Select a measure in the measure lane to seek to its displayed start, including
in a draft preview. Playback keeps its current playing/paused state; automatic
selection during map updates does not seek. Clicking waveform space selects the
containing measure while seeking to the exact clicked time. Shared endpoints
belong to the following measure; clicks in free time select the uncounted range,
clear measure selection and open Free time tools while seeking to the clicked time.
During playback, selection follows the displayed map at the current source
position, independently of the viewport-follow toggle. Gaps and the exclusive
map end have no playing measure selected. This automatic selection never seeks.
Click a boundary/beat to select its owning measure and
drag the line to change its source position. A mapping selection selects
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
new meter/pulse interpretation to the selected measure, optionally propagating
through contiguous following measures until the next existing meter/pulse/grouping
change or free-time gap. There is no separate selected-end propagation control.
Selected-measure notation uses one **Apply changes**
action, with native validation and Undo, rather than a preview gate. Global
renumbering retains its preview/apply workflow; changing a label alone must not
pretend the existing grid now fits.

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

The desktop exposes these operations:

- Add, move, and delete beat anchors; edit their notated position when a
  detected pulse was missing or mistaken for a subdivision.
- Move a measure boundary, split before a numbered beat to add a boundary,
  and join adjacent measures using the common denominator and summed numerators. Splits preserve
  the original meter on both incomplete results and derive their actual extents.
- Reapplying Meter resets beat spacing within the selected measure. The desktop
  no longer exposes range re-spacing; explicit manual draft creation can still
  divide a chosen span into a specified number of measures.
- Shift the selected and following boundaries by a pulse count, absorbing
  consumed measures at fixed endpoints while retaining authored timing.
- Change meter, pulse interpretation/grouping, and pickup/incomplete duration.
- Mark a measure as free time and reclaim an uncounted range as a new measure.
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
editing in this pass. The explicit Shift left/right operation described below
is a separate, deliberate command.

For between-anchor navigation, interpolate linearly in notated quarter-note
position. Arbitrary anchor spacing represents tempo drift, abrupt changes,
accelerando, and ritardando as a piecewise map. Insert more anchors for finer
within-measure timing. A separate smooth tempo-ramp editor is deferred; no
single global BPM value defines this map. Display derived tempo with an
explicit note unit so compound meter cannot silently change the meaning.

Commit a drag as one undoable edit, not hundreds of pointer updates. Provide
an immediate working preview, validate in Rust, and save only the accepted
valid result. Rejected commands leave the last valid map intact. Undo/redo
history is scoped to mapping, keeps up to 50 edits, and survives screen switching
during the same song session. It is not restored after restart or song replacement.
Musical definitions/identities are restored by Undo; revision and ID-allocation
counters remain monotonic.

## Domain and persistence boundaries

Implemented domain shape (validated in Rust before persistence):

| Entity | Responsibility |
| --- | --- |
| Analysis result | Candidate beat/downbeat source frames, pinned model identity, song/revision/preprocessing provenance, and uncalibrated evidence scores |
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

Version-3 song documents contain a version-1 map; valid version-2 documents
migrate without changing the meaning of sections, controls, loops or position.
Malformed/future-version files remain intact. Native load/update validate the map
against the decoded source rate and frame count. A map supports up to 10,000
measures and 80,000 internal anchors. Meter numerators are 1–99, denominators are
1/2/4/8/16/32/64, and stored rational offsets/durations have numerator at most
65,536 and denominator at most 960. Large inference arrays live in a separate
bounded cache, not in the document or routine playback-status payload.

Manual edits and analyzer evidence remain separate. An analysis job is bound
to song identity, map revision, preprocessing and model identity. Changing the
song or editing the map while analysis runs makes its result a stale candidate,
not an automatic mutation. Reruns require an explicit preview/apply operation;
protected ranges and accepted edits outside the target span survive.

## Automatic analysis scope

The desktop uses pinned `beat-this` 1.1.0 with RTen CPU inference and the
`final0` main-model ONNX conversion recorded in `config/beat-model.json`. Model
files are optional downloads, not bundled weights. Source-aligned mono
preprocessing is implemented; parity with Python and small-model comparison
remain evaluation work. Rhythm Map is a research reference, not a dependency.
No Python runtime is required by the shipped app.

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

## Implementation slices and remaining review

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

The domain, manual editor, analyzer adapter and practice navigation are
implemented. Representative guitar-material evaluation, Python parity,
small-model/resource comparison and verified Linux/Windows runs remain open;
these review points are not all completed acceptance gates. See
[IMPLEMENTATION_STATUS.md](IMPLEMENTATION_STATUS.md) for current evidence.

## Verification contract and remaining acceptance work

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
manual tempo-curve drawing, implicit ripple edits and automatic constrained neural
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
Measure, Join / Split / Shift, and Free time panels. They retain
exact source-frame edits, protection and undo. The Measure panel uses a compact
**Meter** pair (two number inputs separated by a slash), measure kind/actual
duration and **Apply changes**. Meter, pulse, grouping and extent are committed
together directly after validation, without a green preview or second approval.
Errors leave the accepted map and typed form intact for correction; Undo restores
the previous notation and anchors. Protected measures cannot apply notation.

The lock/unlock icon sits beside the measure title and remains available across
all three panels. The Measure panel's **Advanced**, collapsed initially, contains beat grouping,
pulse note value, propagation until the next meter change/gap and exact boundary
position controls. Pulse remains independent of the denominator: 6/8 can use a
dotted-quarter click while its denominator still counts eighth notes.

**Batch tools** follows Create measures in the sidebar. It owns First full measure
number and the existing Preview numbering action, which applies to the song map
without requiring a selected measure. A TODO records future grouping updates for
all measures with a matching time signature; this batch operation is not yet
implemented. Analysis, model setup and draft creation stay in the sidebar.
Selected-measure form state lives in the
cohesive `MeasureTools.svelte` component; map/session ownership stays in the editor,
batch numbering form state lives in `BatchTools.svelte`, and the timeline receives
playback presentation through a Svelte snippet. Draft construction and numbering
keep their preview/apply workflows. Join, split, shift, mark free time and reclaim commit
directly after native validation, with Undo for each operation.

## Join, Split, Shift and free-time editing

Join, Split and Shift sit in one horizontal row separated by vertical rules.
Narrow layouts stack the groups with horizontal separators. All three commit
directly after validation with one Undo; Advanced holds numeric anchor controls.

Join requires adjacent, unprotected measures with the same denominator. Its
numerator is the sum, retaining the denominator without reducing the signature:
3/4 + 4/4 becomes 7/4, and 3/8 + 3/8 becomes 6/8. Outer source bounds, actual
extents and timing anchors are preserved, including an anchor at the former shared
boundary. The first measure's identity and pulse remain; explicit groupings are
concatenated (an ungrouped partner contributes its numerator as a group). The
result is full only when its actual extent fills the combined nominal meter;
otherwise it is incomplete. The existing 99-numerator limit still applies.

Split selects **Split before beat** by numbered pulse. The compact dropdown shows
only beat numbers; fixed/interpolated styling remains on the waveform. Rust
projects eligible internal pulse points, including those hidden by visual
grouping/density, and derives the split's exact source frame and notated duration.
Projected grids and split pickers cap the pulse grid at 512 positions per measure;
custom anchors remain separate exact timing corrections. Both results keep the
original meter/denominator and are incomplete; anchors on either side retain
identity and relative timing. An anchor at the split becomes the shared boundary. Splitting 4/4 at beat 3 creates two
partial 4/4 measures of two quarters each. Joining those adds their numerators,
producing a partial 8/4 of four quarters, per the agreed rule. Splitting a pickup
produces incomplete measures with local beat origins. Numeric anchor controls
remain available under **Advanced — beat anchor positions** in Join / Split / Shift;
waveform dragging is the primary correction method.

**Shift by (pulses)** defaults to two and accepts whole counts from 1 through 64.
The unit is the selected measure's pulse note value, including compound pulses;
it is independent of the meter denominator. Shift left/right moves the selected
measure's start and every subsequent internal boundary through the existing
notated timeline. It is not a constant source-time displacement: tempo changes
and authored timing corrections determine each new source frame.

The affected range's start and the last mapped endpoint stay fixed. For shifts
that do not consume the predecessor, its start remains the range start. Right
extends the preceding extent and raises its numerator, retaining its denominator,
when needed: 4/4 plus two quarter pulses becomes full 6/4. The last surviving
shifted measure is shortened when the endpoint cuts its extent. Left shortens
preceding material (making a remaining partial bar incomplete) and extends the
last measure, increasing that meter when needed. Neither direction creates an
overfull measure. If an extension does not fill a denominator-note unit, the
numerator rounds up and the extended measure is incomplete. Explicit grouping
gets an extra group for the added denominator units. Internal meters, kinds and
actual extents retain their identities and travel with surviving shifted bars; a
remaining partial leading pickup retains a recomputed pickup offset.

Authored anchors retain their identities and source frames, with local offsets
and ownership recalculated. Anchors landing on a new boundary are absorbed into
its timing. Former bar lines become anchors only where needed to preserve a
change in timing slope; uniform measures acquire no extra corrections. Frame
rounding permits a one-source-frame interpolation difference. Detected suggestions
are not inputs to the operation.

Select a measure with a predecessor. Protected measures anywhere in the affected
span and intervening free time reject the shift. Trailing uncounted audio stays
outside the operation. Boundaries reaching or passing a fixed endpoint are
absorbed there, removing measures with no remaining extent. Right can consume
one or several tail measures; left can consume the predecessor and earlier
measures, reaching back as needed through contiguous counted audio. If every
internal cut is absorbed, one measure spans the affected range. Surviving
measures use the fixed outer boundary identities and retain all interior timing
corrections from absorbed bars. Meters grow as needed to avoid overfull notation.
Protection and free-time checks include any earlier measures reached by a left
shift. Shifts exceeding the existing 99-numerator limit or requiring unsupported
custom fractions still reject atomically.
The edit commits directly after native validation, with one Undo restoring the
exact original map, including absorbed measures. An opposite shift does not
recreate consumed measure identities or automatically shrink previously enlarged
nominal meters; use Undo for exact restoration of notation and anchors. If the
selected measure is absorbed, select the surviving measure containing its former
start, without seeking.

Free time leaves audio intact while removing the selected measure's map. Adjacent
uncounted audio is represented as a single gap. Click a gap on the waveform or
choose **Uncounted range** in Free time, select a meter/pulse, then **Create measure
from range**. This creates one full, evenly spaced measure with fresh identities
over that entire range and selects it in the Measure panel. Rust rejects overlap
with any existing measure, including an unprotected one. Empty maps allow the
whole source as a range. Undo restores the original map; reclaiming is a new
authored definition rather than recovery of previously removed timing anchors.

`JoinSplitTools.svelte` owns beat/anchor form state and command parsing;
`ShiftTools.svelte` owns the pulse-count form and shift command binding;
`FreeTimeTools.svelte` owns uncounted-range selection and reclaim form state.
`map-ranges.ts` shares presentation-only gaps from ordered Rust-projected ranges.
The native legacy Respace edit remains for existing domain callers/tests; its
desktop action identity stays valid for old keymaps but is unavailable and has
no UI control.
