# Attached stems in the desktop app

Issue #5 attaches existing separation outputs; model installation and running
Roformer-SW inside the app remain issue #6. Start the desktop app with the usual
native dependencies installed:

```sh
CARGO_HOME="$PWD/.cargo-home" npm run desktop
```

Open a managed library song, choose **Stems** beside Notes and Measure map, then
**Attach stems**. Select the completed output directory containing `manifest.json`,
`prepared.wav` and declared stem files. Existing version-1 prototype outputs from
Roformer-SW or Demucs are accepted when their provenance matches the exact reference
recording. The app validates and copies declared stems into the library. The
prototype directory can subsequently be removed: prepared.wav is attachment
validation evidence, not a playback dependency.

Attachment/replacement/removal pauses at the current position and stays paused.
Replacing an attached set and removing stems require confirmation. Opening or
closing the Stems screen alone does not interrupt playback. Removing stems keeps
the reference recording, map, sections and notes. Every reference replacement removes
its stems, including a replacement that preserves the measure map.

In Practice, knobs adjust gains from 0–100%. Drag vertically, scroll, use arrows,
or double-click to reset to 100%. **M** mutes and **S** solos; multiple solos combine,
and mute wins. Keys **1–6** mute bass, drums, guitar, piano, vocals and other,
respectively; absent roles do nothing. Shortcuts are disabled while typing.

Playback automatically uses Original when all present tracks are at 100%, unmuted
and unsoloed. Any adjustment uses Stems. One fixed headroom factor applies to both
sources; Stems details show its attenuation. Gains/mutes survive reopening and
matching roles retain them when replacing a set. Solo is temporary.

Count-in and Loop are stacked icon buttons. Loop cycles Off → Loop → Loop with
count-in → Off. The final state highlights both loop and hourglass icons and enables
Count-in. Disabling Count-in restores ordinary looping. Without a measure map,
only Off/Loop are available. Count-in remains suppressed during measure-map editing.

Preparation runs before opening stem playback and reports progress. Subsequent opens
reuse file-backed PCM and headroom measurements. Current-rate preparation is stored
inside the owned set's `playback-cache` directory; changing output rate rebuilds it.
Stored hashes describe validated attachment; ordinary opens do not rehash stems.
Cache writers serialize within the process and publish immutable PCM generations;
active readers retain an older generation until they release it, then its files are
removed. Failed preparation retains the previous published cache. File size and
modification time invalidate preparation after typical manual edits.
Edits preserving both are not guaranteed to be detected. Missing/invalid stems show
a warning and use Original, preserving saved mix settings; playback read failures
stop playback with an error.

Native attachment/cache logic lives in `src/stems/prepare/library.rs`. Guarded
managed association publication lives in `src/managed/stems.rs`; controller sessions
and generation checks live in `src/desktop/stems.rs`. Frontend live updates use
`ui/stems-session.ts`: one in-flight command plus the latest desired mix, with a
separate saving debounce. Playback uses the shared source/Rubber Band path proven
in #4. Waveform and analysis continue using the decoded reference Audio, so the
whole desktop app is not duration-independent in memory.

No inference, multiple sets, A/B, history, background attachment, cancellation,
restart recovery or cache-management UI is included. Unexpected termination can
leave unreferenced copies. Ordinary errors preserve the previous association and
attempt cleanup; cleanup failures are reported.

## Implementation verification (2026-10-13)

Generated-audio native checks: 153 passed, with two hardware tests ignored during
ordinary automated execution. Frontend: 200 tests passed; Svelte check reported no
errors or warnings; production frontend build passed. Workspace all-target strict
Clippy with beat-analysis passed, as did Rust formatting and diff whitespace checks.

The explicit desktop-controller physical smoke used isolated generated recordings
and the Mac's KT_USB_AUDIO output at 48 kHz. Original → adjusted bass → Original
produced 94 callbacks, no device errors or clipping, and two underflow frames over
the two transitions. This is hardware execution, not subjective listening approval.
The sandbox could not query CoreAudio; the permitted unsandboxed smoke succeeded.
Linux and Windows execution and desktop visual inspection remain unverified here.

```sh
CARGO_HOME="$PWD/.cargo-home" cargo test --locked --offline --all-targets --features beat-analysis
CARGO_HOME="$PWD/.cargo-home" cargo clippy --locked --offline --workspace --all-targets --features beat-analysis -- -D warnings
npm run check
npm test
npm run build
# Optional physical-device smoke (audible generated tone):
CARGO_HOME="$PWD/.cargo-home" cargo test --locked --offline --lib desktop_stems_hardware_smoke -- --ignored --nocapture
```
