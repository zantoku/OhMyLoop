# First desktop implementation

The desktop app wraps the validated native audio engine with Tauri and Svelte.
The first-pass scope in PLAN.md is implemented: local-song loading, waveform
seeking and zoom, named editable ranges, temporary loops, visible speed/pitch
controls, keyboard navigation, and restored practice settings.

## Boundaries

- `src/project.rs`: versioned song documents, section validation, content
  identity, waveform peaks, and durable writes through temporary files.
- `src/desktop.rs`: controller actor that owns CPAL streams on one thread.
  UI commands are handled away from the device callback. It does not require
  unsafe Send/Sync implementations for native audio objects.
- `src/timing/`: source-frame musical map, validation, edits and projections.
- `src/analysis/`: optional model setup and cancellable subprocess inference.
- `src-tauri`: native file picker, command bridge, and app lifecycle.
- `src/keymap.rs`: bounded, read-only configuration-file loading.
- `ui/actions.ts`, `ui/keymap.ts`, `ui/action-dispatcher.ts`: shared command
  metadata, validated bindings, and execution/availability independent of the view.
- `ui`: Svelte screen, canvas waveforms, range interactions, and shortcuts.

The source timeline remains independent from playback speed and output sample
rate. Sections have stable IDs and can overlap. Temporary loops are separate
document fields. Mutations and status polling do not resend waveform data.
Frontend control updates are serialized and coalesced, and older replies do
not overwrite newer edits. The audio engine owns playback timing.

Saved documents are keyed by the source file's SHA-256 digest, rather than
its pathname. Atomic replacement preserves the last complete document when
a write fails. Invalid or future-version documents are not silently replaced.
The app reopens the last song paused, restoring position and controls. Demo
sections are seeded only on first use, so removing them remains persistent.

## Verification

Local macOS checks cover the existing eleven audio tests plus eight tests
for document restoration, nested sections, transient-preserving waveform peaks,
file identity, invalid document preservation, resampling, and loop-endpoint seek
semantics. One additional Rust test covers bounded keymap file reads and
preservation of invalid configuration. The frontend
suite has thirty-nine passing tests covering coordinates, shortcuts, section/loop
behavior, independent control updates, nested phrase creation, Undo preserving
section identity, and exact dragged boundaries passing browser validation.
Gesture checks cover pointer-anchored wheel zoom with pixel/line/page deltas,
middle-button panning across viewport updates and at song boundaries, pointer
cancellation, and preserved left-button selection/seeking. Minimap checks cover
click navigation, continuous dragging at a fixed zoom, edge clamping, cancellation,
and avoiding seek/loop changes. X clears an active
range during playback without changing saved sections, and is suspended while
editing fields or dialogs. Comma seeks to the exact A start; period seeks to B
and disables looping; Enter rewinds to the song start. Configuration tests cover
platform modifiers, conflict validation, file-error fallback, command availability,
contextual payloads, remapped buttons, synchronized hover/help descriptions,
and corrected-file reloads. A/B shortcuts do nothing without a range or while
editing decimal inputs or dialogs.
Svelte/TypeScript checks report no errors or warnings.

The original practice-only native macOS app built into a roughly 15 MiB bundle.
With the optional analysis runtime, the current bundle is about 25 MiB. The first window
inspection found lower controls falling outside the initial viewport; the
layout was adjusted to give the waveform flexible height and keep transport
and practice controls visible.

Native UI verification on macOS with `KT_USB_AUDIO`:

- Dragging the waveform produced a loop from approximately 1.85159 to 3.50530
  source seconds without starting playback.
- Playback advanced native source position at 75% speed; pitch was changed
  independently to -3 semitones, then playback paused.
- Relaunch restored the loop, 75% speed, -3 semitones, and a paused position
  near 2.405 seconds.
- Saving the exact dragged range created a named "Phrase practice" section.
  The initial attempt exposed HTML step-validation rejecting non-rounded
  source coordinates; numeric range inputs now accept arbitrary precision.
- Selecting Chorus moved the enabled loop to 3–6 seconds while remaining
  paused; zoom changed the viewport to 2×.
- The native picker loaded the generated 44.1 kHz PCM WAV fixture. Space
  started and paused its native playback, with position advancing to about
  7.013 seconds.
- Wheel zoom changed the viewport from 1× to 2× around the pointer, leaving
  the paused position and loop bounds unchanged. X removed the range and
  turned looping off on the generated PCM fixture. The fixture's exact prior
  bounds were then restored, and the previously open song was reopened paused.
  Middle-button dragging has automated component coverage; physical mouse
  verification remains pending because desktop automation supports only
  left-button dragging.
- Left-button minimap dragging moved a 4× viewport across the song and
  remained captured outside both minimap edges, clamping to the source start
  and end. Playback stayed paused at approximately 27.884 source seconds,
  with the temporary loop and playback controls unchanged.
- Pointer focus on the waveform and minimap no longer paints a focus outline.
  A native middle click focused the waveform without changing playback or
  loop bounds; minimap dragging stayed free of focus rings. Tab navigation
  still showed a visible outline on the song-position slider. Timeline input
  tracking handles WebKit's scripted-focus behavior while retaining keyboard
  focus indicators.
- Comma moved the paused native playhead to approximately 39.377 source
  seconds at A. With looping enabled, period moved to B at approximately
  64.390 seconds and turned looping off. Enter then moved to zero. The range,
  speed, and pitch were preserved throughout.
- A temporary on-disk keymap reassigned A to Q, mapped the previous-section
  button to the A jump, and supplied the description "Back to the riff".
  Reloading changed the help row, button label, and hover description together.
  Comma became inactive, Q jumped to A, and the remapped button executed the
  same seek. The unchanged test file was removed and another reload restored
  the defaults. The final native app remains open, paused at A, using defaults.

The window is shown/focused at startup and macOS reopen, and closing the main
window requests app exit. This addresses the background-without-visible-window
state encountered during repeated automation relaunches. The rebuilt app was
left open, paused, on the song that was open before the gesture checks.

Linux and Windows builds and tests are configured in the desktop CI workflow.
This revision has not been verified on those platforms. Code signing,
installer distribution, automatic device reconnection, streaming long files,
and sample-accurate synchronization through adaptive stretching remain later
work. None of the musical practice features deferred in PLAN.md have been
added to this pass.

## Measure mapping extension

The dedicated mapping editor, practice ruler, optional model installer and real
inference worker are now implemented. Architecture, exact setup/evaluation commands,
current test counts, native QA and remaining validation are maintained in
[IMPLEMENTATION_STATUS.md](IMPLEMENTATION_STATUS.md). The workflow contract is in
[MEASURE_MAPPING.md](MEASURE_MAPPING.md); analysis is a best-effort draft.
