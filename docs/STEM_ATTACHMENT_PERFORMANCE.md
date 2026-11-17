# Stem attachment: retain validated playback preparation

Attachment previously validated prototype audio into temporary PCM, discarded that
PCM, decoded copied stems again, then prepared the reference and stems yet again
for playback. The desktop now copies declared stems first and validates those exact
owned bytes once. Manifest/job identity and prepared.wav evidence still come from
the prototype. That validation's PCM, resampling and common headroom factor become
the persistent playback cache, without another decode or scan.

The source-path adapter is private to attachment; the CLI/listening harness keeps
its existing behaviour. Cache adoption uses immutable generations and existing
reader ownership. Exclusive temporary PCM can move into the cache; cross-volume
paths use bounded copying. Input size/mtime is captured before validation, compared
after validation and transfer, and checked immediately before cache publication.
The cache key retains that original evidence. Edits during preparation cannot be
silently recaptured as newer metadata describing older PCM. Ordinary failures clean
partial output and preserve the previously attached association/cache.

The maintainer also selected a whole-track peak cutoff of **−60 dBFS** for new
attachments: omit tracks whose maximum absolute decoded sample across both channels
is strictly below **0.001**. Hash, finite samples and full geometry are checked before
that decision. Peak is measured during the existing decode, so no detection pass is
added. A brief sample at or above the cutoff retains the track. Excluded owned copies
are removed; prototype audio and the reference remain untouched. Excluded tracks
skip resampling, headroom and cache work and do not appear in the mixer. The checklist
keeps a “Skipped” row; existing attached sets are not pruned when reopened. If every
track is below the cutoff, attachment fails with “No audible stems found” and keeps
the previous set.

The checklist follows actual copy → owned validation/alignment → headroom → cache
adoption → association publication. No repeated cache decoding is listed as work.
Skipped rows remain skipped after completion, and overall progress reaches 100%
only after successful association publication.

## Reproduction and measurements

The generated fixture benchmark calls the same native attachment/cache functions,
using six non-quiet stereo float WAV stems at 44.1 kHz and preparing playback at
48 kHz. It streams fixture generation without whole-song RAM allocation. Generation,
compilation, the folder picker and desktop association/player setup are excluded
from the reported attachment times. These are local macOS measurements, not an
inference or listening-quality benchmark.

```sh
CARGO_HOME="$PWD/.cargo-home" cargo run --locked --offline --example stem_attach_benchmark -- 15
CARGO_HOME="$PWD/.cargo-home" cargo run --locked --offline --example stem_attach_benchmark -- 300
CARGO_HOME="$PWD/.cargo-home" cargo run --locked --offline --release --example stem_attach_benchmark -- 15
CARGO_HOME="$PWD/.cargo-home" cargo run --locked --offline --release --example stem_attach_benchmark -- 300
```

| Generated duration / build | Before | After | Reduction |
| --- | ---: | ---: | ---: |
| 15 s / debug | 8.422 s | 4.056 s | 52% |
| 300 s / debug | 167.165 s | 84.074 s | 50% |
| 15 s / release | 1.056 s | 0.542 s | 49% |
| 300 s / release | 19.135 s | 10.060 s | 47% |

Inputs are identical across each comparison: reference SHA-256 is
`eb3f33d2c26c4332474494b2e58232de7b259506786a6002ef5d2c7255ebfc52`
for 15 s and
`77f0abc04613e037e38e4923760a53491b5a382305c01bdd8d37c985f1381022`
for 300 s. Every fixture stem remains above the omission cutoff. Prepared playback
stays at 40,320,000 / 806,400,000 bytes respectively. Warm cache lookup after
attachment takes about 1–2 ms locally.

| Work over six retained stems | Before | After |
| --- | ---: | ---: |
| Audio decode passes | 21 | 8 |
| Resampling passes | 14 | 7 |
| Headroom scans | 2 | 1 |
| Content-hash passes | 14 | 8 |
| Authoritative stem copies | 6 | 6 |

For the 300 s fixture, hashed bytes fall from 1,481,760,952 to 846,720,544. Owned
validation checks the copied bytes directly, rather than hashing prototype stems
and copied stems separately. Timing is reported by the benchmark; regressions assert
pass counts, byte-identical prepared output/headroom and cache reuse, rather than
flaky wall-clock thresholds.

A stage probe after optimization on the 15 s debug fixture measured 1.723 s
resampling, 1.297 s hashing, 0.690 s decoding, 0.306 s headroom and 0.043 s copying.
Per-sample decoder writes are not the dominant remaining cost, so this change does
not rewrite PCM writing or broadly optimize the app's development profile. Actual
recordings, codecs, storage and device rates can produce different times; debug
builds remain substantially slower than release builds.

Separate fresh-process `/usr/bin/time -l` runs observed maximum RSS of 10,354,688
bytes for 15 s/debug and 8,650,752 bytes for 300 s/release, including fixture generation.
Both are comfortably below the 256 MiB preparation target. These samples corroborate
bounded buffers rather than establishing an allocator proof for every codec/platform.
The sandbox blocks the system resource counter; permitted unsandboxed measurement
was required. No user recordings or library were changed. Linux and Windows execution
was not measured.

Validation completed with 169 native tests passing (two physical-device tests skipped
in the normal run), strict workspace/all-target Clippy with beat-analysis, formatting
and whitespace checks. Existing actual-DSP gain/transport tests remain included.
