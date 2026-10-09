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

## Optional Beat This! analysis

The desktop enables `beat-this` **1.1.0** (MIT), a Rust port by danigb of
[Beat This!](https://github.com/CPJKU/beat_this) by the Institute of Computational
Perception, JKU Linz. CPU inference uses RTen (MIT OR Apache-2.0); downloading
uses reqwest (MIT OR Apache-2.0). The terminal player does not enable this
feature by default.

The optional upstream `final0` checkpoint and its main-model ONNX conversion
are recorded separately in [config/beat-model.json](config/beat-model.json):
original project, conversion revision, exact download URLs and SHA-256 hashes.
Models are downloaded from the Rust port's pinned official artifacts, rather
than bundled in Git or the app. The upstream work and port use MIT terms;
[licenses/beat-this-MIT.txt](licenses/beat-this-MIT.txt) retains both notices.
The app bundles that notice and provenance, and successful model installation
copies them beside the weights as `LICENSE.txt` and `PROVENANCE.json`.

Model timestamps use OhMyLoop's own source-aligned mono preprocessing;
this is identified independently of the model. Runtime licensing, converted
weight provenance and analysis accuracy are separate concerns. Binary-release
attribution for the entire resolved dependency graph remains a release task.
