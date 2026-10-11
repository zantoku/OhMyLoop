# OhMyLoop

A free, open-source music practice player for Linux, macOS, and Windows.
The first desktop implementation provides a waveform workspace, editable
sections, temporary loops, independent speed and pitch controls, editable measure
maps with optional beat analysis, authored-map metronome and practice-state restoration. The validated terminal prototype remains available.

The prototype uses Rust, CPAL, Symphonia, and bundled Rubber Band 4.0.0.
Playback uses Rubber Band's R3/Finer engine. R2 remains available to the
measurement probe for comparison, but is not exposed as a playback option.

## Desktop app

Install Rust stable (1.89 or newer), Node.js 22 or newer, and your platform's native build
tools (listed below). From the project directory:

```sh
export CARGO_HOME="$PWD/.cargo-home"
npm ci
npm run desktop
```

Development builds optimize inference and resampling dependencies while keeping
app code debuggable. The first build after these profile changes takes longer;
restart `npm run desktop` to pick up native code and build-profile changes.

Build a native app locally:

```sh
npm run desktop:build
# macOS app-only build:
npm run desktop:build -- --bundles app
```

The macOS bundle is generated at `target/release/bundle/macos/OhMyLoop.app`.
Windows and Linux use their platform's Tauri bundle formats. Linux also needs
the WebKitGTK and desktop integration packages listed in
[Tauri's prerequisites](https://v2.tauri.app/start/prerequisites/).

Use **Library → Open song**, or try the built-in demo. Click the waveform to seek; drag to
choose a temporary loop. The A/B handles and numeric fields adjust the bounds.
Scroll over the waveform to zoom at the pointer; hold the middle mouse button
and drag to pan. Click or drag the whole-song minimap to move the detailed
view while keeping its zoom level. Zoom buttons also let you adjust the view.

Named sections have explicit start/end ranges and may overlap. Selecting a
section prepares its range and seeks to its start while preserving play/pause
and loop on/off state. Adjusting a temporary loop does not modify the saved
section. Sections can be renamed, resized, and deleted; Undo restores deleted
sections during the current app session.

Space toggles playback, L toggles looping, and A/B set loop boundaries.
Comma jumps to A, period jumps to B, and Enter rewinds to the song start.
Jumping to B turns looping off; seeking preserves play/pause state.
X clears the A–B range and turns looping off, brackets navigate sections,
N adds a section, and Cmd/Ctrl+O opens a song. Arrow keys
seek five seconds; Shift+arrows seek 0.1 seconds. Shortcuts are suspended while
editing fields or dialogs. The in-app shortcut panel lists these controls.

Shortcuts, command-button mappings, and hover/help descriptions come from a
shared action model. Optional `keymap.json` overrides load from the app's
configuration directory and can be reloaded from **Shortcuts → Customize
shortcuts**. See [docs/KEYMAP.md](docs/KEYMAP.md) for the format, file locations,
validation rules, and [example configuration](config/keymap.example.json).

Practice documents are saved in the app's per-user data directory, under
`practice`. They contain stable section IDs, source coordinates, controls,
loop selection, last position, and plain-text song notes. A SHA-256 identity lets settings survive
moving or renaming an unchanged audio file. Playback is restored paused.
Corrupt or unsupported documents are reported and left intact. For development
only, `OHMYLOOP_DATA_DIR` selects the practice-document directory itself; it does
not redirect the model or keymap folders. The analysis cache is its sibling
`analysis` folder. Use `OHMYLOOP_MODEL_DIR` and `OHMYLOOP_KEYMAP_FILE` separately
for isolated model/configuration paths.

Audio-device failures leave the song editable and provide a Retry audio
action. Sample-rate mismatches use delay-compensated, band-limited resampling.
The playhead follows native output-frame positions rather than a frontend
playback timer; adaptive stretching and physical device latency still make
it an estimate.

`npm run dev` opens a frontend-only development preview with demo fixtures.
Audio, analysis and native musical-map edits require `npm run desktop`.

## Music library

Open **Library** from the header to find previously opened songs. Real songs are
added automatically on successful open; the built-in demo is excluded. Artist,
title and album come from embedded tags when available, with the filename as a
fallback title. **Edit → Save details** changes the library record without writing
to your audio file. Later opens retain your edits.

Search title, artist, album or file location. Click a column heading or use Sort by
to choose any of the six fields, including Date added and Last opened; reverse the
order with the adjacent button. The default is Last opened, descending. Click a
song title to reopen its existing practice state. Opening the library pauses
playback. The top-right navigation button becomes **Practice** while browsing.
**Open song** lives in the library heading; Cmd/Ctrl+O also opens through the
library, keeping it visible if the file picker is cancelled.

The catalogue is `practice/library.json`, separate from each song's practice
JSON and the analysis cache. It uses the same content hashes, so reopening a moved
unchanged file updates its location and keeps its annotations. Missing or replaced
files produce an error; use Open song to locate a moved file. Previously used
songs join the catalogue the next time they are opened or restored. No folder
scan/import, artwork, removal or export workflow is implemented yet.
See [docs/LIBRARY.md](docs/LIBRARY.md) for details.

## Song notes

Open **Notes**, to the left of Measure map above the practice waveform, for a
plain-text editor. Notes are saved with the song, including line breaks and
whitespace. The **X** beside the Song notes title saves and returns to practice; **Library** stays
at the top right. Edits autosave, and normal app quit flushes pending text.
Failed writes retain the text and offer Retry save. Markdown rendering is deferred.
See [docs/NOTES.md](docs/NOTES.md) for persistence and lifecycle details.

## Measure mapping

Open **Measure map** above the practice waveform; the **X** beside its title returns to the
player. **Library** stays in the top-right corner. Both screens share transport and retain their own viewports.

In **Measure map**, **Metronome** defaults on to play the displayed authored map,
including a green draft before applying it. **Click level** adjusts its volume;
clicks follow the chosen pulse and accent measure/group starts. **Follow playhead**
also defaults on in both screens and pages the waveform during playback without
changing zoom. Each screen retains its own follow preference.
Panning, minimap navigation, zoom and view-fitting preserve this setting; only
the Follow playhead toggle turns it off. Returning to practice uses accepted-map
clicks and preserves the shared metronome enable/level preference.
Left, Right and Enter also reveal the playhead in
the waveform while paused, preserving zoom. See [map review behavior and timing limits](docs/MEASURE_MAPPING.md#listening-to-the-displayed-map).

The **Analysis** sidebar holds Analyze song and **Show detected beats**. Raw
suggestions hide by default once a map or draft is displayed; the checkbox can
restore them. Beat markers thin as you zoom out and regain detail when you zoom
in. Play/pause and speed sit in a compact waveform overlay. The panel below the
waveform holds measure timing, notation, beat-anchor and range-editing tools.
Selecting a measure in the timeline seeks to its start while preserving the
current playing/paused state, including when reviewing a draft.
Clicking waveform space selects the containing measure and seeks to the clicked
time. Selection follows the current measure during playback. Moving a measure
boundary evenly spaces beats in both adjacent measures; Undo restores the
previous boundary and anchors.
The Measure panel uses a compact meter pair and one **Apply changes** action with
Undo. Its **Advanced** fold-out contains pulse, grouping and exact boundary
positions; the title lock protects the measure. **Batch tools**, below Create
measures, holds global numbering and batch beat grouping, both with Preview/Apply.
Choose a time signature and groups such as `2+2+3` for 7/8, or leave grouping blank
to clear it. Grouping applies throughout the song to unlocked measures with that
exact signature, including pickups and partial measures; the form reports changed
and skipped locked measures. It preserves corrected timing and pulse values and
creates one Undo step. Pulse controls click spacing; grouping accents clicks only
where group starts coincide with a pulse.
**Join / Split / Shift** lays out all three tools horizontally with vertical
separators. Join sums numerators over a common denominator; Split before beat
shows only beat numbers and produces two incomplete measures retaining the prior
meter (4/4 split at beat 3 becomes two partial 4/4 bars). Numeric
anchor controls are under Advanced. **Free time** can remove a measure's map or
create a new, evenly spaced measure over a selected uncounted waveform range.
Shift left/right moves the selected and remaining measure boundaries by a chosen
number of the selected measure’s pulses, preserving source beat timing and outer
endpoints. Right expands the preceding meter; left makes it partial and extends
the final measure. Measures consumed at an endpoint are absorbed into surviving
bars. Each operation supports Undo. Range re-spacing is no longer a desktop tool.

**Create measures** provides two explicit sources:

- **Detected beats** is selected after analysis. Choose the time signature and the
  note value of each detected pulse, then set the first downbeat. Its position
  snaps to the nearest detected beat in the selected span. **Preview detected
  measures** calculates the number of full/partial measures and places their
  boundaries at detected source frames. **Beat timing tolerance ε (ms)** defaults
  to 30 ms: internal beats interpolate unless timing anchors are needed to stay
  within that error of detection. Set 0 ms for an exact fit. The summary shows
  retained anchors. Review the green draft, then **Apply map**. Changing draft
  settings requires another preview.
- **Even spacing (manual)** uses a span and an explicit measure count. It ignores
  detected beats and is available without installing or running a model.

**Span** expands the target range and optional grouping controls. Detected drafts
end at the last observed pulse; leading/trailing audio can remain unmapped. Meter
changes, pickups and partial bars can then be corrected locally. Draft settings
and the selected measure's notation controls are independent.

Drag measure/beat lines to correct timing, or use exact source positions and
arrow-key nudges (10 ms; Shift gives 1 ms). The inspector supports meter changes,
pickups, incomplete bars, beat anchors, splits/merges, free-time gaps and protected
corrections. **Clear map** removes all measures/anchors, including protected ones;
**Undo** restores them. It retains detected beats for rebuilding and keeps practice
sections, loops and playback settings. Mapping undo/redo retains up to 50 edits
during the same song session and survives switching back to practice; it is not
saved across restarts. The accepted map is
persisted with the song in version-4 documents; valid version-2 and version-3 documents migrate.
Practice uses a single waveform ruler: measure numbers and vertical barlines in
mapped audio, timecodes in free time. The separate section lane stays. Loops
snap to whole measures by default; turn off **Snap to measures** or Shift-drag
for free placement. Loop handles and A/B shortcuts use barlines; exact time
inputs and saved section ranges remain precise. Both screens share metronome
on/off and click level, defaulting to on/50%. Practice follows the accepted map;
mapping auditions the displayed draft. Measure navigation remains in the footer.

**Analysis model → Download analysis model** optionally installs the pinned
Beat This! model (about 83 MB). **Analyze song** runs CPU inference in a separate,
cancellable process. Download progress shows received bytes; analysis shows
per-stage progress, completed segments and elapsed time. The model folder is
shown beneath Verify / reinstall. Downbeat conflicts produce review warnings.
Analysis never applies a map automatically; protected spans block replacement.
Installed models work offline, and manual mapping needs no download or Python
runtime. For development/offline provisioning, `OHMYLOOP_MODEL_DIR` selects a
folder containing the two pinned ONNX files listed in
[config/beat-model.json](config/beat-model.json); checksums are verified before
inference. The normal model folder is `models/beat-this-v1` under app data.

Analysis is limited to one hour and its accuracy remains provisional. Generated
fixtures showed missed/extra beats and significant downbeat errors with changing
meter. Real guitar recordings, Python parity and the small-model comparison still
need evaluation. See [verification and limits](docs/IMPLEMENTATION_STATUS.md),
[the mapping contract](docs/MEASURE_MAPPING.md), and
[technology research](docs/BEAT_MAPPING_RESEARCH.md). Count-ins, inter-loop rests,
speed ladders, section automation, sequences, export UI, recording, stems and tabs
remain later work in [PLAN.md](PLAN.md).

Desktop checks:

```sh
npm run check
npm test
npm run build
cargo test --locked --release -p ohmyloop --features beat-analysis
cargo clippy --locked --workspace --all-targets -- -D warnings
```

Run native Cargo builds and tests sequentially. The Tauri CLI sets macOS
deployment build variables, so concurrent CLI builds can invalidate artifacts
while the separate Rust documentation test process is reading them.

## Terminal prototype: build and run

Install Rust stable and your platform's C++ build tools:

- macOS: Xcode Command Line Tools (`xcode-select --install`).
- Linux: C++ compiler, `pkg-config`, and ALSA headers; on Debian/Ubuntu,
  `sudo apt-get install build-essential pkg-config libasound2-dev`.
- Windows: Visual Studio Build Tools with Desktop development with C++
  and the Windows SDK; use Rust's MSVC target.

No system Rubber Band installation, Python runtime, FFmpeg, or web frontend
is needed for the terminal player. Rust 1.99.0 was used for local validation; the
manifest declares Rust 1.89 as the language/dependency minimum.

From the project directory:

```sh
# This workspace already has dependencies cached here. Optional elsewhere.
export CARGO_HOME="$PWD/.cargo-home"
cargo run --locked --release -- play --demo
```

For your own song:

```sh
cargo run --locked --release -- play "/path/to/song.mp3"
```

The player starts paused. Supported input codecs include WAV/PCM, FLAC, MP3,
Vorbis, AAC-LC, and ALAC through Symphonia. Mono is duplicated to stereo;
multichannel input is rejected instead of silently downmixing it.

## Terminal controls

```text
play
pause
speed 0.5
pitch -5
seek 10
mark verse 12.5
mark chorus 34.2
markers
jump chorus
loop verse chorus
loop 12.5 18.75 5
loop off
status
save practice.ohmyloop.json
load practice.ohmyloop.json
quit
```

- Speed is continuous from 0.25 to 2.0; 0.5 means half speed.
- Pitch is continuous from -12 to +12 semitones; fractional values allow tuning.
- Markers and loop bounds use seconds in the **original song**, regardless of
  speed. `mark NAME` without a time uses the current estimated output position.
- Loop bounds can be marker names or numbers. Minimum loop length: 100 ms.
- Optional loop seam fade is in milliseconds, 0–50; default 5. Paired fades
  avoid shortening the selected period, but can cause a small volume dip.
- While a loop is enabled, seeking outside it returns to its start. Use
  `loop off` to jump elsewhere in the song.
- Marker names and session paths entered in the REPL are single tokens.
  The song path passed at launch may contain spaces when quoted by your shell.
- Sessions hold settings and markers, not copies of the audio. They must be
  loaded with the same song identifier used when saving.

## What to listen for

Use `play --demo`, then issue each command separately:

1. `play`: establish the normal rhythm and notes.
2. `speed 0.5`: the beat and chord changes should take twice as long. The notes
   should retain their pitch; the bass should not drop an octave.
3. `pitch -5`: the notes should move lower, while the beat stays at half speed.
4. `loop 1.17 2.03`: hear several repeats. Listen for sharp clicks, crackles,
   stuttering, or unexpected gaps at the repeat boundary.
5. `loop off`, `speed 1`, `pitch 0`, `seek 0`: return to the reference sound.

Some texture changes at extreme settings are expected. Try a real song next:
listen for softened drum attacks, warbling sustained notes, muddy bass, and
vocal coloration. Those are subjective quality judgments the tone tests do
not establish. The synthetic demo is original material generated by this app.

## Measurements and tests

```sh
cargo fmt --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked --release
cargo run --locked --release -- probe artifacts/probe
cargo run --locked --release -- smoke --demo
python3 scripts/check_vendor.py
```

`probe` needs no audio device. It measures duration, steady sine-wave pitch,
and computation time, and creates JSON plus listening WAVs. The command exits
successfully only when the supported R3 checks pass. R2 comparison failures
remain visible in the output and JSON. The pitch estimator uses zero crossings
on a steady sine wave; it is not a general pitch detector or quality metric.

`smoke` needs an audio device and plays five seconds of generated material.
It changes tempo/pitch, seeks, and enables a loop. Its PASS means callbacks and
transport operations ran without a reported device error; inspect the underrun
count too. It does not mean subjective quality or physical latency was tested.

For optional codec coverage, install FFmpeg and run:

```sh
python3 scripts/check_codecs.py
```

The cross-platform CI workflow builds, lints, tests, and uploads probe artifacts
on Linux, macOS, and Windows. It does not require sound hardware. CI must run
on a GitHub repository before its configured platforms can be marked verified.

## Render listening files without a device

```sh
cargo run --locked --release -- demo artifacts/demo.wav
cargo run --locked --release -- render --demo artifacts/slow.wav --speed 0.5
cargo run --locked --release -- render --demo artifacts/lower.wav --pitch -5
cargo run --locked --release -- render --demo artifacts/loop.wav --loop 1.17 2.03 --seconds 8
```

Create the output directory first if it does not exist. `render` writes stereo
32-bit float WAV; `--seconds` limits **output** duration. Looped rendering
requires that limit. `--start` seeks in source seconds. `--fade` controls the
loop seam fade. Normal rendering stops at the end of the source audio.

## Architecture and limits

```text
Symphonia decode → stereo source PCM → Rubber Band worker → bounded queue → CPAL callback
                                             ↑
                             terminal commands and saved sessions
```

The device callback only reads the preallocated queue, converts samples,
applies short fades, and updates atomics. File access, DSP, allocations, and
command acknowledgements happen elsewhere. The worker prebuffers roughly
50 ms, invalidates old queue generations on seek, and re-primes the stretcher
with its prescribed padding and delay compensation. Loop playback wraps the
source reader without resetting DSP history on every repeat.

This prototype decodes the whole song into memory, capped at roughly 512 MiB
of stereo PCM. It prefers an output device configuration at the source rate.
If unavailable, it uses offline band-limited rate conversion. Device loss is
reported; the desktop app can reopen the default device with Retry audio.
Settings outside the tested ranges are rejected.

`status` reports nominal source positions attached to submitted output frames,
plus the source reader position. Rubber Band's adaptive timing, especially
after live speed changes, makes the output coordinate approximate. Physical
device latency is not subtracted. The 50 ms queue is not a measured bound on
end-to-end command latency. Sample-accurate waveform/playhead synchronization
remains a production task.

See [docs/RISKS.md](docs/RISKS.md) for measured outcomes and outstanding risks.
Project instructions live in [AGENTS.md](AGENTS.md). Licensing and upstream
attribution are in [LICENSE](LICENSE) and [THIRD_PARTY.md](THIRD_PARTY.md).
