# Measure mapping: current behavior and implementation contract

Drafted: 2026-10-09. Updated: 2026-11-19 (screen redesign, concept MB). Status: manual editing, map-review
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

**Measure map** is one of the song's views in the header (Practice · Measure map
· Stems); the view switcher returns to Practice, so the screen has no close X. The screen
follows the chosen design, concept MB, a floating toolbar
([brief](design/measure-map.md), layout in [UI_DESIGN.md](UI_DESIGN.md#measure-map-concept-mb)).
Every command belongs to the shared action/keymap catalog, with descriptions and
configurable bindings.

The app shell owns the loaded song, document, and transport. Switching screens
does not reload audio or change play/pause, speed, pitch, or loop settings.
Each screen retains its own viewport. Share transport controls and timeline
navigation primitives; keep mapping gestures separate from loop gestures.

The mapping screen has:

- A compact readout band: the measure and beat of the displayed map or draft
  (time in free time and without a map), Undo, Redo, and once a map exists
  **Redraft** and **Song settings** (numbering, grouping, clear map).
- A whole-song minimap with click/drag navigation, plus a detailed waveform
  with pointer-anchored wheel zoom and middle-button panning. Its ruler shows
  measure numbers (time in free time); barlines run through the waveform, beats
  are dotted lines across its full height, and detected beats are amber marks
  along the bottom. A draft is dashed green; free time is hatched.
- A **floating toolbar** at the selected measure, just below the ruler: measure
  number, properties (`4/4 · Full ▾`), Split, Join, Shift ◀ ▶, Lock and ⋯. Its
  panels hold the measure's properties and the expert controls. The selected
  measure's barlines carry grips and its beats carry diamonds; both drag.
- A bar below the waveform with the **Detected beats** toggle, a legend, and
  **Fit measure** and **Whole song**.
- The **drafting drawer** on the right while the map is empty (or while
  redrafting): find the beats, read them, preview and apply; or draft by hand.
- The playback bar: previous/next measure, Play, **Go to m.**, a compact speed
  control, **Turn looping off** when a loop is on, and what the click follows.
  Pitch controls remain on Practice.

Click the waveform to select the containing measure while seeking to the exact
clicked time; this works in a draft preview too. Shared endpoints belong to the
following measure; clicks in free time select the uncounted range (the toolbar
then offers **Count as a measure**) while seeking to the clicked time. **Go to
m.** seeks to a measure's start and selects it. Playback keeps its current
playing/paused state; automatic selection during map updates does not seek.
During playback, selection follows the displayed map at the current source
position, including free time, and the toolbar moves with it. This automatic
selection never seeks.
Drag a barline to change its source position; dragging the selected measure's own
end keeps it selected. A mapping selection selects map elements, not an A–B
loop. Seeking follows the existing outside-loop rule; mapping controls must not
silently replace the practice loop. **Turn looping off** appears in the playback
bar while a loop is on, and Esc turns it off too.

Keyboard selection/nudging, exact position inputs, and visible keyboard focus
support the same edits as dragging. Validation failures are announced. Commands
route through scoped action availability so loop-editing commands are not
accidentally dispatched from this screen. Existing keymap files remain valid;
the catalog grows with backward-compatible defaults and explicit contexts.

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

Build measures for a selected span with a preview. On an empty map a detected
draft spans the whole song; when redrafting an existing map, the drawer asks
for the span to replace (**Replace from** / **Replace until**, at the playhead or
as exact times), so locked corrections outside it survive. The user may instead
set two measure boundaries and specify how many measures fit between them; this
creates an editable evenly spaced draft. Both methods work locally and leave
accepted material outside the selected span unchanged. A span that reaches into
a locked measure is rejected, and the drawer says so in place.

Detected measure endpoints remain exact source-frame positions. Within each
measure, fit evenly spaced notated pulse positions between the endpoints. Only
add a detected timing anchor when interpolation misses a pulse by more than
epsilon. Recheck each resulting segment after adding an anchor, so the final
piecewise map stays within epsilon at every detected pulse. This also handles the
actual duration of a final partial measure. Retained anchors keep the detected
source positions; collinear pulses need no anchors even at zero epsilon.

**Beat timing tolerance ε (ms)** is an editable number field under **Tolerance**
in the drafting drawer's preview step, validated from 0 through 1000 ms. Its initial 30 ms value is
a tunable starting point, not a claim about detector accuracy. Tolerance is in
original source time, independent of playback speed/pitch. It does not alter raw
detections or existing manual corrections, and is session draft state rather than
a saved-song schema change. Only applying a rebuilt draft changes the map.

Meter changes occur at explicit measure boundaries. The properties panel applies a
new meter/pulse interpretation to the selected measure, optionally propagating
through contiguous following measures until the next existing meter/pulse/grouping
change or free-time gap. (**Also the following measures, until the next change**). There is no
separate selected-end propagation control. Selected-measure notation uses one
**Apply** action, with native validation and Undo, rather than a preview gate. Global
renumbering and batch grouping retain their preview/apply workflow; changing a label alone must not
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

Version-1 reference-timeline records contain the version-1 map inside the UUID
song envelope; notes and speed/pitch controls remain song-level. The UI uses a
flattened version-5 document. The authorized pre-release launch reset replaces
legacy identity migration; malformed/future UUID files remain intact. See
[UUID identity](UUID_IDENTITY.md). Native load/update validate the map
against the decoded source rate and frame count. A map supports up to 10,000
measures and 80,000 internal anchors. Meter numerators are 1–99, denominators are
1/2/4/8/16/32/64, and stored rational offsets/durations have numerator at most
65,536 and denominator at most 960. Large inference arrays live in a separate
bounded cache, not in the document or routine playback-status payload.

Saved detection is read once per song/editor session through a separate native
cache-read command. Validate the cache version, model/preprocessing identity,
reference asset/timeline revision, source rate/frame count and event positions
before displaying it, rebinding verified cached evidence to the current session. A
missing cache is normal; unreadable/invalid caches report an error and stay intact.
The installed model is not needed to reload existing evidence. Keep existing
suggestions visible during reanalysis/cancellation/failure and replace them only
with a successful result for this song. Ignore late cache replies after a new job,
song change or disposal. A changed map revision does not invalidate source-aligned
suggestions; it still prompts review before using them.

Manual edits and analyzer evidence remain separate. An analysis job is bound
to song identity, map revision, preprocessing and model identity. Changing the
song or editing the map while analysis runs makes its result a stale candidate,
not an automatic mutation. Reruns require an explicit preview/apply operation over a chosen span (the
whole song only on an empty map); protected ranges and accepted edits outside
the target span survive, and a span that reaches a protected measure is
rejected with an explanation.

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

Practice keeps its saved-section lane and a single ruler within the waveform.
Accepted measure numbers replace timecodes over mapped audio; barlines extend
vertically through the waveform. Number labels thin when zoomed out, while
barlines remain visible. There is no separate practice measure lane. Unmapped
and free-time audio retains timecodes and displays no invented musical position.
The musical footer provides previous/next measure and Go to measure through
shared map lookup and action metadata.

**Snap to measures** defaults on as a session preference. Creating a waveform
loop selects the whole measures touched by the drag (in either direction);
ending exactly on a barline does not add the following measure. Loop handles
snap to the nearest barline, and their arrow keys move to the previous/next
boundary. Shift-drag bypasses snapping; Shift-arrow keeps fine adjustment.
The A/B shortcuts snap the playhead to the nearest barline. With no existing
range, A/B or enabling Loop selects the containing mapped measure. Free-time
endpoints, exact seconds inputs and selecting saved sections remain precise.
Snapping never modifies an existing loop merely by toggling the preference or
changing the map. The existing 100 ms minimum loop length still applies.

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

The drafting drawer leads the empty map and steps aside once a map exists;
**Redraft** reopens it. **From detected beats** is the default route:

1. **Find the beats:** the one-time model download (with its size, progress and
   Cancel, and a link to Settings › Beat analysis), **Analyze song** with
   progress and Cancel, or the number of beats found with **Analyze again**.
   While stems are being separated, analysis waits (they share one processing
   slot) and the drawer says so, also beside a disabled **Analyze again**;
   drafting by hand still works. After the map changed since the analysis, the
   drawer says the suggestions may be stale. Once a map exists and the drawer
   is closed, the bar below the waveform keeps such problems visible as a quiet
   note ("Analysis out of date · Redraft…", "Beat analysis failed", "Analysis
   waits while the song is split into stems").
2. **Read the beats:** time signature, the note one beat is, and where the first
   full measure starts (an exact time, or **Playhead**). When redrafting an
   existing map, also the span to replace.
3. **Preview and apply:** **Preview draft**, then the summary (measures, full
   and partial bars, timing corrections, ε), **Discard** or **Apply map**;
   **Tolerance** holds ε.

**By hand** marks a span (start and end at the playhead or as exact times),
counts its measures with a meter and beat note, and previews an evenly spaced
draft, without detected pulses. New suggestions switch the drawer to detected
beats and set the first full measure at the first detected downbeat; the draft
snaps it to the nearest detected pulse. A changed input requires a new preview
before Apply. The inputs, and those the shown draft was previewed with, are
session state: they survive closing the drawer and switching screens, so a
draft previewed before leaving can still be applied. A numbering or grouping
preview from Song settings is applied or discarded in the bar below the
waveform even while the drawer is open; the drawer only applies its own drafts.

**Clear map** (Song settings › Start over, after an inline confirmation) is an
explicit, undoable operation that removes all accepted measure
and beat annotations, including protected ones, and resets numbering to 1. It
preserves source identity, monotonic IDs and detected evidence for rebuilding.
Practice sections, loops, speed/pitch and play/pause state are independent. If
only an unapplied preview exists, Clear map discards it without a save.

## Listening to the displayed map

On the mapping screen the metronome is always on at full level (design decision
7); the playback bar says whether the click follows the map or the draft. Practice
keeps its own Click on/off and level, which the Measure map neither shows nor
changes; they default to on/50% for the app session.
Clicks begin once an authored map or draft exists, and stop while audio output is lost. The metronome uses the green
draft while one is displayed, otherwise the accepted map. Applying retains that
timing; discarding returns to
the accepted map. Raw detected suggestions never drive clicks directly. Measure
starts receive a distinct accent, grouping starts a secondary accent, and the
chosen pulse note value determines the remaining clicks. Pickups keep their
notated offset; incomplete measures end early and free-time gaps are silent.
Arbitrary beat anchors change interpolation without becoming extra pulses.

Practice also offers **Count-in** after an accepted map has been built, with
**each** (count in before each loop repeat) as a separate segment of the Loop
button; the two are independent. Count-in uses only
the accepted map, shares Click level, and sounds independently of the metronome.
The controls are absent from measure mapping. See the README Count-in section
for local tempo, phase, free-time and cancellation rules. Both toggle buttons
highlight their on state.

Rust builds the bounded pulse schedule from original source frames, converts it
to the playback device rate, and mixes short fixed-pitch clicks after Rubber Band
on the render worker. Speed changes alter spacing while pitch changes leave the
click sound alone. Seeking resets click history; loop wraps restart the schedule
at the rounded source loop start. Pause stops both song and clicks. Live map/level
changes take effect after the existing bounded audio queue (about 50 ms, plus
device latency); no frontend timer schedules audio and the callback is unchanged.
Mixing reserves volume-dependent headroom for clicks to avoid clipping.

The readout follows the displayed map too. The review map is session state; it
does not change the saved song schema. Leaving mapping restores accepted-map clicks
at Practice's click setting and transport coordinates. An unapplied draft stays in the editor
and can be auditioned again on returning to mapping.

Both screens always follow playback (design decision 4; the `map_follow` toggle
is removed), with the same rule (`followPage` in `ui/timeline.ts`): the view pages
when playback starts off-page, when the playhead crosses the page's right edge,
and on backwards seeks and loop wraps, preserving zoom. The final page clamps to
EOF. A manual pan, minimap move, wheel zoom, Fit measure or Whole song holds
until the playhead next leaves the page. Paused editing keeps the viewport still.
Changing screens preserves each viewport.
Left/Right navigate strictly earlier/later mapped measure starts, across free-time
gaps. Within a measure, Left returns to its start; at its start, Left goes to the
preceding measure. At the first/last start, an arrow with no target does nothing.
Enter jumps to the earliest mapped measure, including a pickup. An absent or empty
map retains five-second arrow seeks and Enter at source zero. Shift+arrows retain
0.1-second seeks. These rules also apply to arrows on the focused position slider.
Explicit playhead navigation (including Left, Right and Enter) reveals the
resulting position in either screen even while paused or with following off.
It preserves zoom, and does not move the view if the
resulting position is already visible. Rewinding also works when the playhead
is already at the rewind target but the view has been panned elsewhere.

Clicks share the engine's nominal source-position estimate. Rubber Band's adaptive
timing can move individual transients around that estimate, particularly during
live speed changes; this is not a claim of sample-accurate click/transient alignment.
Count-ins, practice rests and exported clicks remain outside this review feature.

## Map review detail and editing layout

**Detected beats** in the bar below the waveform (D) controls only the raw
suggestion overlay. Suggestions show initially, then hide by default when an
authored map or green draft appears. An explicit choice wins for the rest of that
song session, including screen switches. Hiding suggestions does not delete them
or change draft construction, the accepted map, protection or the metronome.

Beat lines and barlines thin at discrete density levels: every 2nd, 4th or 10th,
with larger steps for very long songs, and a note in the waveform says so.
Detected marks obey the same rule. They remain intact in the map; sampling keeps
its phase while panning, and zooming restores detail. Measure numbers thin like
Practice's ruler. The selected measure's barlines always show. Its grips and beat
diamonds need room: below about 28 px they hide until Fit measure (F) zooms in.
This is a presentation rule only.

The **floating toolbar** (`MapToolbar.svelte`) floats just below the ruler,
centred on the selected measure, with a notch pointing at it. Against an edge of
the view it stays fully inside the waveform and the notch slides to stay over the
visible part of the measure; once the measure scrolls out of view it collapses to
an edge tab ("m. 37") on that side, which shows the measure again (map_fit, F).
It never stays open for a hidden measure. The geometry is a pure function
(`ui/map-toolbar.ts`). The toolbar fades while a line is dragged. Its contents,
in order: the measure number; properties (`4/4 · Full ▾`, M); **Split at beat n**
(S); **Join** (J); Shift ◀ ▶ (`[` `]`); Lock (L); ⋯. Unavailable tools stay
focusable and say why in their tooltip. During a preview the toolbar hides: a
draft is for listening.

**Measure properties** (the ▾ panel): meter, kind (Full, Pickup, Incomplete),
how long a pickup or incomplete measure lasts, grouping, the beat note ("Click
on"), and **Also the following measures, until the next change**, with Cancel and
**Apply**. Meter, pulse, grouping and extent are committed together directly
after validation, without a green preview or second approval. A rejection is
announced in the panel, which stays open with the typed values; Undo restores the
previous notation and anchors. A locked measure's panel says so and cannot apply.
Pulse remains independent of the denominator: 6/8 can use a dotted-quarter click
while its denominator still counts eighth notes.

**⋯ panel** (expert controls next to the tools they belong to):
**Exact positions** (barline before and after, as `m:ss.mmm` or seconds; each
commits as one edit), **Beats** (choose a beat to see or type its position;
**Anchor at playhead** anchors the chosen beat, or the beat nearest the playhead;
**Remove anchor**), **Shift** (the shift step, re-snapping and ε) and **Mark as
free time**. Choosing a beat diamond on the waveform chooses it here too.

**Song settings** (`SongMapDialog.svelte`) is a modal dialog with a leading X:
**Measure numbering** (first full measure number, Preview), **Beat grouping**
(time signature, grouping, changed/skipped counts, Preview) and **Start over**
(Clear map…, with an inline confirmation). Previews close the dialog so the
green draft shows on the waveform with Apply and Discard in the bar below it.
**Beat grouping** lists the song's distinct numerator/denominator pairs and
accepts denominator-note groups such as 2+2+3 in 7/8. A blank field clears
explicit grouping. **Preview** replaces grouping on all unlocked measures with
that exact signature throughout the song, across meter changes and free-time
gaps, including pickups and incomplete measures. Matching locked measures are
skipped and reported. The dialog shows changed/skipped counts and disables
preview when nothing changes. Native validation requires positive whole-number
groups summing to the numerator; invalid edits keep the dialog open with the
accepted map and typed input. Preview/Apply/Discard and one Undo/Redo step use
the existing mapping session. Grouping changes preserve source boundaries, fixed
beat anchors, measure identities, kind, extent, offset and pulse values. The
displayed grouping grid changes; metronome click times stay fixed, and group
starts accent only clicks coinciding with the existing pulse. Grouping already
belongs to the version-1 map, so no persistence migration is needed.

A rejected toolbar or drag edit is announced over the waveform ("Not changed: …",
with a dismiss X); the map stays as it was. Rejections from a panel, the drafting
drawer or Song settings show there instead, beside what was typed.

Ownership: `MeasureMapScreen.svelte` owns the selection, view, panels and the
sessions (`MappingSession` for edits and history, `AnalysisSession` for analysis);
`MapWaveform.svelte` draws and drags; `MeasureProperties`, `MeasureExpert`,
`FreeTimeCount` and `DraftDrawer` own their form state; `measure-tools.ts` holds
the toolbar's availability rules and `draft-steps.ts` the drawer's steps. Draft
construction, numbering and batch grouping keep their preview/apply workflows.
Join, split, shift, mark free time and counting free time commit directly after
native validation, with Undo for each operation.

## Join, Split, Shift and free-time editing

Split, Join and Shift sit in the floating toolbar. All three commit directly
after validation with one Undo; the ⋯ panel holds numeric anchor controls.

Join requires adjacent, unprotected measures with the same denominator. Its
numerator is the sum, retaining the denominator without reducing the signature:
3/4 + 4/4 becomes 7/4, and 3/8 + 3/8 becomes 6/8. Outer source bounds, actual
extents and timing anchors are preserved, including an anchor at the former shared
boundary. The first measure's identity and pulse remain; explicit groupings are
concatenated (an ungrouped partner contributes its numerator as a group). The
result is full only when its actual extent fills the combined nominal meter;
otherwise it is incomplete. The existing 99-numerator limit still applies.

**Split** splits before the inner beat nearest the playhead, so it needs no
input; the button names that beat ("Split at beat 3") and hovering or focusing it
draws a preview line there. It is unavailable while the playhead is outside the
selected measure. A toast confirms the split, says when later measures were
renumbered, and offers Undo. Rust
projects eligible internal pulse points, including those hidden by visual
grouping/density, and derives the split's exact source frame and notated duration.
Projected grids and split pickers cap the pulse grid at 512 positions per measure;
custom anchors remain separate exact timing corrections. Both results keep the
original meter/denominator and are incomplete; anchors on either side retain
identity and relative timing. An anchor at the split becomes the shared boundary. Splitting 4/4 at beat 3 creates two
partial 4/4 measures of two quarters each. Joining those adds their numerators,
producing a partial 8/4 of four quarters, per the agreed rule. Splitting a pickup
produces incomplete measures with local beat origins. Numeric beat positions are
in the ⋯ panel; dragging the diamonds is the primary correction method.

The **shift step** (⋯ panel) defaults to one pulse and accepts whole counts from 1 through 64.
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
are not inputs when re-snapping is off.

**Re-snap to detected beats** is initially on. Saved detection is restored when
the mapping screen opens, including after an app restart, without running analysis
again. Without detected beats the checkbox is unavailable and shifts keep authored
timing, as the panel says (changed in the redesign: Shift used to be unavailable
until re-snapping was turned off). A shift first repartitions the authored map as above, then snaps each
new internal barline to its nearest detected pulse (ties choose the earlier pulse).
The affected outer endpoints stay fixed, preserving the preceding map and free-time
edges. Existing anchors in the affected measures are replaced, including anchors
from former barlines. Meters, pickup offsets, actual extents and surviving measure
identities remain those produced by the shift.

The ε (ms) input shares the drafting drawer's tolerance, initially 30 ms;
changing either field updates the other. Pair the detected pulses strictly inside
each resulting measure with its notated pulse positions in order, including pickup
phase and fractional extents. Keep only the corrections needed by the same
largest-residual, segment-rechecking fitter used for initial creation. Epsilon
bounds the error at every matched pulse, rather than the sum of errors. At 0 ms,
all pulses fit exactly; collinear pulses still need no fixed anchor. Grouping does
not change the underlying pulse interval used for fitting.

If snapping collapses a measure or the detected interior pulse count disagrees
with its notated count, reject the entire shift with an explanation. Review the
detection, adjust the map/pulse setting, or turn re-snap off; do not guess across
missing/extra pulses. Protection, free-time and other shift checks still apply.
Both shifting and refitting are one direct edit and one Undo restores all original
boundaries and anchors. The checkbox and shared epsilon are song/editor-session UI settings, retained
when selecting other measures, entering free time or changing screens;
no document schema change is needed. Native `Edit::Shift` accepts an optional
`resnap: { pulses, epsilon_ms }`; omitted settings preserve the previous behavior.

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

**Mark as free time** (⋯ panel) leaves audio intact while removing the selected
measure's map, and selects the resulting range. Adjacent uncounted audio is
represented as a single gap. Click a gap on the waveform to select it: it is
outlined and the toolbar shows its extent with **Count as a measure ▾**, whose
panel takes a meter and beat note (initially those of the removed or neighbouring
measure) and **Create measure**. This creates one full, evenly spaced measure
with fresh identities over that entire range and selects it. Rust rejects overlap
with any existing measure, including an unprotected one. Empty maps allow the
whole source as a range. Undo restores the original map; reclaiming is a new
authored definition rather than recovery of previously removed timing anchors.

`MeasureExpert.svelte` owns the exact-position and beat form state;
the shift step, re-snapping and ε are screen session state bound into it;
`FreeTimeCount.svelte` owns the reclaim form.
`map-ranges.ts` shares presentation-only gaps from ordered Rust-projected ranges.
The native legacy Respace edit remains for existing domain callers/tests; its
desktop action identity stays valid for old keymaps but is unavailable and has
no UI control.
