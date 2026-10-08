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
- `src-tauri`: native file picker, command bridge, and app lifecycle.
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

Local macOS checks cover the existing eleven audio tests plus seven tests
for document restoration, nested sections, transient-preserving waveform peaks,
file identity, invalid document preservation, and resampling. The frontend
suite has eighteen passing tests covering coordinates, shortcuts, section/loop
behavior, independent control updates, nested phrase creation, Undo preserving
section identity, and exact dragged boundaries passing browser validation.
Gesture checks cover pointer-anchored wheel zoom with pixel/line/page deltas,
middle-button panning across viewport updates and at song boundaries, pointer
cancellation, and preserved left-button selection/seeking. X clears an active
range during playback without changing saved sections, and is suspended while
editing fields or dialogs. Svelte/TypeScript checks report no errors or warnings.

The native macOS app builds into a roughly 15 MiB app bundle. The first window
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

The window is shown/focused at startup and macOS reopen, and closing the main
window requests app exit. This addresses the background-without-visible-window
state encountered during repeated automation relaunches. The rebuilt app was
left open, paused, on the song that was open before the gesture checks.

Linux and Windows builds and tests are configured in the desktop CI workflow.
They have not been executed locally and are not yet verified. Code signing,
installer distribution, automatic device reconnection, streaming long files,
and sample-accurate synchronization through adaptive stretching remain later
work. None of the musical practice features deferred in PLAN.md have been
added to this pass.
