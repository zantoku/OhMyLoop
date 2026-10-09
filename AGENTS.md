# OhMyLoop contributor guide

## Product and scope

OhMyLoop is a free, open-source music practice desktop app for Linux, macOS,
and Windows. Its core features are independent playback speed and pitch,
named song-section markers, precise seeking, and repeatable loops.

The audio risk prototype is validated. The current milestone is the first
single-song desktop implementation described in PLAN.md. Preserve the CLI
prototype and its audio tests while building the desktop UI.

## Technical direction

- Use Rust for the playback engine, CPAL for device output, Symphonia for file
  decoding, and Rubber Band for independent tempo and pitch processing.
- Keep the engine independent of the UI. Use Tauri with Svelte/TypeScript for
  the desktop UI. Native audio and musical timing remain in Rust.
- Use GPL-3.0-or-later for project code. Preserve third-party source, license,
  and attribution notices. Rubber Band is GPL-2.0-or-later.
- Keep development tools and caches inside the workspace when the environment
  does not allow installation elsewhere. Do not commit generated tools or builds.

## Audio invariants

- Markers and loop bounds refer to source audio frames, independent of speed.
- The audio callback must not allocate, block, access files, or communicate with
  the UI through blocking operations. Decode and process on a worker thread.
- Bound queued audio so commands remain responsive. Track rendered and audible
  positions separately, including time-stretcher and device latency.
- Account for Rubber Band's padding, delay, and variable input/output sizes.
- Flush stale audio when seeking. Treat DSP history and loop transitions
  explicitly; provide a short crossfade or paired fades to prevent clicks.
- Clamp and validate controls. Explain practical quality and latency limits.

## Verification and reporting

- Test actual DSP output for pitch and duration independently, and verify seek
  and loop boundaries. Use generated audio so tests need no copyrighted music.
- Exercise device playback on available hardware; distinguish automated checks,
  hardware checks, and subjective listening results.
- Build and test on all three platforms in CI. A configured CI job is not proof
  that a platform has passed; report untested platforms honestly.
- Run formatting, linting, and focused tests appropriate to each change.
- Document exact setup and run commands, architecture, dependencies, and known
  risks. Do not claim technical risks are settled without evidence.

## Working practices

- Treat clean architecture and maintainability as requirements. Keep domain
  models, persistence, action execution, and UI presentation separate; prefer
  small, cohesive modules and explicit interfaces over growing event handlers.
  Centralize action identities, input/button bindings, and help descriptions so
  behavior, tooltips, and shortcut documentation stay consistent. Validate
  versioned configuration at its boundary and preserve invalid user files.
- Prefer small modules, explicit state, and narrowly scoped unsafe FFI.
- Follow the first-pass scope in PLAN.md. Defer count-ins, musical rests, tempo
  detection, speed ladders, sequences, export UI, recording, stems, and tabs.
- Persist stable section identities and source coordinates. Keep temporary
  loops independent from saved sections. Version saved song documents.
- Preserve user changes and keep generated assets reproducible.
