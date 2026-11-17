# OhMyLoop product and implementation plan

Recorded: 2026-10-08.
Updated: 2026-10-12.
Validated prototype baseline: `1ada9f4` — Build validated Rust audio practice prototype.
Desktop/keymap baseline: `3f48398` — Add configurable actions and refine timeline navigation.

## Product direction

OhMyLoop is a free, open-source desktop music practice app for Linux, macOS,
and Windows, primarily focused on guitar. Short phrases and longer song
sections are equally important practice workflows.

The core is a song player with independent speed and pitch controls, section
markers for jumping through a song, and freely selectable loops. The user
reports excellent sound quality from the Rust/Rubber Band R3 prototype and
has approved this technical direction.

Rust, CPAL, Symphonia, and Rubber Band remain the audio foundation. Tauri with
a Svelte/TypeScript interface is the desktop UI stack. Project code
uses GPL-3.0-or-later. Detailed prototype evidence and remaining technical
risks are recorded in `docs/RISKS.md`.

## First implementation pass: a complete single-song workflow

Implemented; evidence and remaining platform limits are in
[docs/DESKTOP.md](docs/DESKTOP.md). The desktop player provides:

- Open a local song and display its waveform.
- Play, pause, and seek by interacting with the waveform.
- Create and name sections; select a section and jump to its start.
- Select a temporary loop anywhere, including a short phrase within a section.
- Keep independent speed and pitch controls visible and easy to adjust.
- Make navigation and transport usable from the keyboard, especially while
  holding a guitar.
- Automatically save and restore the song's section definitions and practice
  settings.

Named sections and temporary loops should coexist. A guitarist should be able
to navigate a whole song, then isolate a few difficult notes without having
to restructure the saved sections.

The implementation uses Tauri and Svelte/TypeScript with the native Rust
engine. Playhead synchronization and broader Linux/Windows/device validation
remain evidence-driven work; completed local checks do not settle all risks.

## Current implementation: editable measure mapping

The dedicated **Measure map** screen opens from above the practice waveform and has its
own minimap, waveform and musical ruler. Both screens share the loaded song and
transport; switching retains practice settings and each viewport.

Optional Beat This! main-model analysis uses the pinned Rust port with RTen CPU
inference, model verification, progress and cancellation. Detected beats/downbeats
are suggestions: the user chooses meter, pulse and first downbeat. Detected drafts
derive measure counts and observed boundaries; epsilon controls whether internal
beats need timing anchors. Manual evenly spaced drafts work without a model.
Draft construction, global numbering and batch grouping use Preview/Apply; other accepted map edits
commit directly after validation, with Undo/Redo.

The full-width editor provides **Measure**, **Join / Split / Shift**, and **Free
time** tabs. Meter edits reset internal spacing; Advanced holds grouping, pulse,
propagation and precise timing. A title lock protects each measure. Join sums
numerators over a common denominator, and a numbered split creates two partial
measures with the original meter. Explicit pulse-based shifts realign later bar
lines while retaining authored timing and fixed endpoints; consumed measures are
absorbed and extended meters grow instead of becoming overfull. Optional detected-beat
re-snapping rebuilds affected timing using the shared creation epsilon. Free-time ranges
can be reclaimed as one new, evenly spaced measure. Batch tools contains global
numbering and grouping for all unlocked measures sharing an exact time signature.
Batch grouping preserves authored timing and pulse values, with Preview/Apply,
changed/skipped counts and one Undo step.

Authored maps support tempo/meter changes, odd/compound meters, pickups, incomplete
measures and free time. Resizing a boundary evenly spaces both attached measures.
The displayed draft/map can be reviewed with a metronome and paged playhead
following, both initially on. Optional detected-beat visibility and marker density
reduce clutter. Mapping history is session-only; the accepted version-1 map is
saved in version-1 reference-timeline records inside the UUID song envelope.
The flattened UI practice document is version 5; incompatible pre-UUID metadata
is reset on launch instead of migrated. Practice displays the
accepted ruler and measure navigation. Automatic replacement preserves material
outside its target span and rejects protected conflicts.

Detailed scope, domain model, work packages, and acceptance evidence:
[docs/MEASURE_MAPPING.md](docs/MEASURE_MAPPING.md). Technical research and
primary sources: [docs/BEAT_MAPPING_RESEARCH.md](docs/BEAT_MAPPING_RESEARCH.md).
The first editor and optional analyzer are implemented. Local verification,
provisional analysis results and remaining evaluation are in [docs/IMPLEMENTATION_STATUS.md](docs/IMPLEMENTATION_STATUS.md).

The remaining features below are outside the measure-mapping pass. The map
will supply their musical timing foundation.

## Current implementation: music library

The first catalogue workflow adds real songs when they are opened, seeds artist,
title and album from embedded tags, and preserves library edits thereafter. Store
canonical file location, UTC date added and last opened; sort the list by all six
criteria in either direction, with default recent-first order. Search metadata
and file locations. A Library/Practice header button switches screens. Open song lives in the library
heading, and selecting a
song returns to practice with its existing annotations.

Library entries now use stable song UUIDs. Audio assets and reference timelines
have separate IDs; hashes verify file contents and analysis inputs. The versioned
library index remains separate from the authoritative song envelope. Opening a
moved unchanged file can update its location; Import separate song deliberately
creates independent notes and maps even for identical audio. Explicit replacement
keeps the song UUID and either preserves compatible timing or clears its positional
annotations for changed timing. Each song has one current recording/map. Library replacement recovers changed/missing files after restart.
Do not modify embedded tags or copy audio in this identity slice.
Leave directory scans, batch imports, artwork, playlists, removal and portable
project export outside this pass. Detailed behavior and persistence are in
[docs/LIBRARY.md](docs/LIBRARY.md).

## Next implementation: managed library (#3)

Scope agreed 2026-10-13. See
[the implementation plan](docs/MANAGED_LIBRARY_IMPLEMENTATION_PLAN.md) for the
complete product decisions and acceptance criteria. Copy imports directly into a
user-selected library, preserve relative links and UUIDs, check hashes on song
open, and provide confirmed permanent deletion only in the Library edit panel.
Setup occurs on first import; library creation/opening lives in Library settings.
No journal/recovery, locks, staging/verification phase, trash/restore, storage-size
UI, import cancellation or in-app relocation. Move the folder with the app closed
and reopen it. Stems remain later work.

## Current implementation: song notes

A Notes button left of Measure map opens a plain-text song editor. Autosave,
explicit close, library navigation and normal app exit retain pending edits;
failed saves keep the text for retry. Notes and mapping use an X beside their white-panel titles to
return to practice and retain Library at the top right. Library still uses
Practice at the top right. Notes are song-level data in the UUID song envelope;
positional annotations belong to reference timelines. Incompatible pre-UUID
metadata is reset on launch, as authorized by the sole user. Defer Markdown, rich
text and separate notes export. See [docs/NOTES.md](docs/NOTES.md).

## Musical timing and loop practice controls

### Count-in

Implemented (2026-10-11): Practice provides an optional one-measure count-in
before Play starts and a separate checkbox for count-in before every loop
repetition. It requires an accepted measure map, uses the local meter,
pulse/grouping and anchor tempo at the start, and follows effective playback
speed. Manual Play/Space starts include one silent mapped pulse before the
clicks; loop repetitions keep the immediate one-measure count-in. Count-in uses
the shared click sound/level even with metronome off.
Pickups and arbitrary starts preserve phase; free time uses the nearest mapped
frame. The render worker inserts clicks into the output queue while keeping
source coordinates fixed; the device callback uses scalar status updates.
Pause/restart, seeks, disabling and pitch/speed behavior have generated-audio
regressions. See README.md for details and native smoke commands.

### Pause between repetitions

Provide an optional pause of **one measure** between repetitions of a loop.
This is a timed rest between passes, distinct from the transport's manual
pause. Whether the rest includes an audible click remains a UX decision.

### Musical timing foundation

The measure-mapping implementation introduces best-effort beat/downbeat analysis
and an editable source-to-musical-time map. Tempo and meter may change within
a song; a single steady BPM is no longer the proposed foundation. The accepted
map, including user corrections and pulse interpretation, owns musical timing.

BPM alone does not define a measure. Count-ins resolve the local timing as
documented above. Future rests must resolve the local meter, pulse/grouping
and source map at the loop boundary; their behavior at tempo/meter changes or
free-time boundaries remains to be specified.

Count-in timing rule (implemented); proposed for inter-loop rests: follow the effective
playback tempo, including speed changes. For a source at 120 BPM played at
0.5x, the effective tempo is 60 BPM; a four-quarter-note measure lasts four
seconds. Inter-loop rests remain deferred.

### Speed ladder

Allow a practice loop to run for a configurable number of repetitions, with
a configurable speed increase **between successive repetitions**.

- Track repetition progress explicitly.
- Apply speed changes at repetition boundaries.
- Keep pitch independent as speed increases.
- Integrate count-in and the optional one-measure rest with the same schedule.

Still to decide: starting and maximum speed, how increments are expressed
(percentage points, relative percent, or BPM), and what happens when the
configured repetition count or speed limit is reached. Do not silently turn
this requirement into multiple repetitions at each speed; that would be a
separate option to discuss.

## Later: section settings, playback programming, and export

### Per-section playback settings

Allow individual **speed, volume, and pitch** settings for each section
marker/section. Associate settings with stable section identities so renaming
or moving a section does not lose its configuration.

Decision for the first desktop pass: named sections have explicit editable
start/end ranges. Short phrases may overlap or sit inside longer sections.
Section selection prepares that range and seeks to its start; looping remains
an independent on/off choice. Temporary loops do not modify saved sections.

### Playback sequences

Allow the user to program an ordered sequence of sections for playback.
Sequences should reference saved sections by identity rather than copying
their labels or relying on list positions.

Still to decide: repetition rules within a sequence, step-specific overrides,
transition behavior, and interaction with loop rests and speed ladders.

### Audio export

Export pitch-shifted and time-stretched audio. Reuse the same playback
definitions and DSP configuration for live playback and offline rendering.
Account for section-specific volume when rendering programmed playback.

Still to decide: export scope (whole song, selected section/loop, or sequence),
file formats, and whether count-ins and inter-repetition rests are included.

## Stretch goals

- **Record a play-along performance.** Capture the guitarist's input while
  playing the backing audio; preserve alignment and account for input/output
  latency.
- **Transcription and tablature.** Provide transcription features and a tab
  editor synchronized with the audio waveform.

These are future goals, with no commitment to a particular implementation,
model, service, or delivery order yet.

## Planned integration: managed songs and stems

The user has authorized architectural planning for a managed library, stable
UUID song identity, local stem separation and synchronized practice mixing.
See [docs/STEMS.md](docs/STEMS.md) for ADRs, launch reset, storage ownership,
reference-timeline rules, UI and dependency-ordered delivery slices. UUID
identity/reset/replacement and managed copying are implemented. The bounded native
stem playback proof is implemented, reviewed and accepted after maintainer
listening; desktop integration remains planned. Roformer-SW is the selected
separation model.

Song UUIDs replace content hashes as identity; hashes remain asset integrity and
inference provenance. The user-selected managed library copies imported audio
and holds generated sets. Measure mapping and timing analysis stay on the
reference recording regardless of the audible mix. Deliver UUID identity and
managed imports before attaching stems; prove shared practice DSP before mixer
UI and integrated model setup. The sole user authorized resetting incompatible
legacy app metadata on launch instead of migrating annotations. Preserve audio,
prototype outputs, new UUID data and existing CLI behavior.

## Architectural direction

These are implementation guidelines that accommodate the agreed roadmap;
they are not additional first-pass features.

### Separate song definitions from a running practice session

Keep song identity, markers, section ranges, analysis results, and saved
settings separate from transient transport state such as current position,
active loop, repetition number, and current ladder speed.

Use stable IDs and versioned persistence. Keep coordinates tied to the
original source audio; tempo and pitch changes must not move stored section
boundaries or future transcription annotations. Define how source coordinates
map to beats and to output time instead of mixing those units.

Plan for global playback defaults with optional per-section overrides. The
inheritance and precedence rules should be explicit when section settings and
sequence editing are introduced.

### Give the Rust engine ownership of practice timing

Keep musical scheduling independent of the UI and device callback. Design
the transport to accommodate states such as count-in, section playback,
inter-repetition rest, next repetition/sequence step, and completion.

Repetition boundaries should be explicit events. They can advance the ladder,
apply upcoming section settings, insert a rest, or advance a sequence. Use
the audio clock for these transitions, with enough lookahead to prepare the
next audio segment; frontend timers must not determine audible timing.

Preserve the existing callback invariants: no allocation, file access, or
blocking work in the device callback. Keep DSP and command handling on the
worker side, and retain generation-based invalidation for seeking.

### Share the playback definition with offline rendering

Represent a practice arrangement in a way that both live playback and export
can consume. Share scheduling rules, source selection, and settings while
allowing separate live and offline rendering modes. This avoids independently
implementing speed ladders, section settings, and sequence behavior twice.

### Leave room for additional synchronized material

Keep audio assets, analysis jobs, and timeline annotations separate from
transport controls. Future recordings and stems need a common source timeline
and mix model. Future tabs need an explicit mapping among source time, beats,
and notation positions. Expensive analysis and separation jobs should run
outside the playback worker and report progress to the UI.

## Scope discipline and next decisions

The editable measure-mapping workflow in docs/MEASURE_MAPPING.md is implemented
with automatic draft generation and measure navigation. Next, evaluate it on real
guitar material and gather cross-platform and user-workflow feedback. Batch
grouping of matching meters within a song is implemented. Count-ins, rests,
ladders, section settings, sequences, export UI, recording and transcription
remain later work. Stems and managed-library integration now have an authorized
design plan; implementation is staged separately in docs/STEMS.md.

Keep strengthening timing invariants with generated fixtures and evaluating
automatic suggestions against independent evidence. Preserve both phrase practice
and longer-section navigation in the existing practice screen. Keep analyzer
evidence, accepted musical definitions, persistence, and playback scheduling
separate.


Practice refinement (2026-10-11): the section lane remains, with one waveform
ruler showing measure numbers/barlines in mapped audio and timecodes in free time.
Loop creation defaults to whole measures; handles/A-B use boundaries, with a
snap toggle, Shift override and exact time inputs for phrases. Practice and
mapping share metronome enable/level; practice follows the accepted map and
mapping can audition its displayed draft. Both screens provide default-on paged
playhead following with independent toggles; manual navigation preserves them.
Preferences are session state.
