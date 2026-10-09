# OhMyLoop product and implementation plan

Recorded: 2026-10-08.
Updated: 2026-10-09.
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

## Next implementation pass: editable measure mapping

Create a dedicated **Measure map** screen, opened from a button in the practice
screen's header. Give it its own minimap, detailed waveform and musical ruler,
with tools for building and correcting a measure map. Share the loaded song
and transport so screen changes do not reset playback or practice settings.

Use Beat This! through the existing Rust port as the first automatic-draft
candidate. The user defines meter, pulse note value and the first full
downbeat; detected beats/downbeats are suggestions. Derive measure numbering
from the accepted map, accounting for tempo variation, time-signature changes,
odd/compound meters, pickups, incomplete measures and free-time spans.

Keep a complete manual workflow available without a model. Provide local
boundary/beat edits, selected-range measure construction, meter changes,
protected corrections and undo/redo. Automatic reruns preview candidates and
must preserve accepted edits. Persist a versioned map anchored to source audio
and add measure navigation/ruler display to the practice screen.

Detailed scope, proposed domain model, work packages, and acceptance evidence:
[docs/MEASURE_MAPPING.md](docs/MEASURE_MAPPING.md). Technical research and
primary sources: [docs/BEAT_MAPPING_RESEARCH.md](docs/BEAT_MAPPING_RESEARCH.md).
Implementation is in progress; the current pause checkpoint and remaining
verification are in [docs/IMPLEMENTATION_STATUS.md](docs/IMPLEMENTATION_STATUS.md).

The remaining features below are outside the measure-mapping pass. The map
will supply their musical timing foundation.

## Later: musical timing and loop practice controls

### Count-in

Provide an optional count-in before loop playback. The count-in must be
scheduled by the audio engine so it stays synchronized with playback.

Still to decide: count-in length, sound, and whether it occurs only before
the first repetition, before every repetition, or according to a user option.

### Pause between repetitions

Provide an optional pause of **one measure** between repetitions of a loop.
This is a timed rest between passes, distinct from the transport's manual
pause. Whether the rest includes an audible click remains a UX decision.

### Musical timing foundation

The next measure-mapping pass introduces best-effort beat/downbeat analysis
and an editable source-to-musical-time map. Tempo and meter may change within
a song; a single steady BPM is no longer the proposed foundation. The accepted
map, including user corrections and pulse interpretation, owns musical timing.

BPM alone does not define a measure. Future count-ins/rests must resolve the
local meter, pulse/grouping and source map at the relevant loop boundary.
Behavior at a meter/tempo change or free-time boundary remains to be specified
when those scheduling features are implemented.

Proposed timing rule: count-in and inter-loop rests follow the effective
playback tempo, including speed changes. For a source at 120 BPM played at
0.5x, the effective tempo is 60 BPM; a four-quarter-note measure lasts four
seconds. Confirm this behavior before implementing the musical timing UI.

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
- **Stem separation.** Generate separate, synchronized stems and make them
  available for practice playback and mixing.
- **Transcription and tablature.** Provide transcription features and a tab
  editor synchronized with the audio waveform.

These are future goals, with no commitment to a particular implementation,
model, service, or delivery order yet.

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

The next deliverable is the editable measure-mapping workflow described in
docs/MEASURE_MAPPING.md, including automatic draft generation and measure
navigation. Count-ins, rests, ladders, section settings, sequences, export UI,
recording, stems, and transcription remain later work.

Finalize timing invariants with generated fixtures, then build the manual
editor before wiring automatic suggestions. Preserve both phrase practice
and longer-section navigation in the existing practice screen. Keep analyzer
evidence, accepted musical definitions, persistence, and playback scheduling
separate.
