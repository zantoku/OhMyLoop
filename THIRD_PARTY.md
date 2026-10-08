# Third-party software

OhMyLoop project code is GPL-3.0-or-later; see `LICENSE`.

## Rubber Band 4.0.0

Copyright 2007–2024 Particular Programs Ltd, t/a Breakfast Quay.
License: GPL-2.0-or-later (used here under GPLv3).
Source: https://github.com/breakfastquay/rubberband
Pinned commit: `e4296ac80b1170018a110bc326fd0d45a0eb27d6`.

The library's source, headers, single compilation unit, documentation, and
notices are bundled in `vendor/rubberband`. No upstream code was changed.
`UPSTREAM.json` records the included files and their original Git blob hashes;
`python3 scripts/check_vendor.py` verifies them. Unused integrations and tests
are omitted. The bundled build uses the upstream built-in resampler and FFT,
with Apple's Accelerate FFT on macOS. It does not require FFTW or a separately
installed Rubber Band library.

Bundled upstream auxiliary sources retain their notices:

- Julien Pommier's SIMD math functions: zlib license, embedded in their headers.
- Erik de Castro Lopo's float conversion header: BSD-style notice in the header.
- Speex resampler: BSD-style license in `src/ext/speex/COPYING` (not enabled).
- KISS FFT: BSD-3-Clause notices in `src/ext/kissfft` (not enabled).

## Rust dependencies

Direct dependencies: CPAL (Apache-2.0), Symphonia (MPL-2.0), crossbeam-queue
(MIT OR Apache-2.0), anyhow (MIT OR Apache-2.0), serde and serde_json
(MIT OR Apache-2.0), hound (Apache-2.0), and cc (MIT OR Apache-2.0).
The desktop implementation additionally uses Rubato (MIT), sha2 (MIT OR
Apache-2.0), Tauri and its dialog plugin (MIT OR Apache-2.0), and Svelte (MIT).
The JavaScript Tauri API uses MIT OR Apache-2.0. Vite, Vitest, and the frontend
test tools are development dependencies; `package-lock.json` pins that graph.
`Cargo.lock` pins the full dependency graph, including platform-specific crates.
Source and license files are available in the Cargo registry cache and the
upstream package repositories. Packaging a binary release must include the
applicable license and attribution files from the resolved dependency graph.
