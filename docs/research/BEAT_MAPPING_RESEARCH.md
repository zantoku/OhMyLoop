# Measure mapping research for OhMyLoop

Research date: 2026-10-09. Scope: existing analysis technology, deployment,
licensing, and editable musical-time precedents. This is research, not an
implementation plan. No candidate was installed, built, benchmarked, or tested
against OhMyLoop recordings during this pass. Upstream timings and accuracy
figures below are reports by their authors, not local measurements.

Implementation follow-up: OhMyLoop now uses the pinned Rust port/main-model
conversion with an authored-map editor. This document preserves the original
research snapshot; current pins, measured results and remaining evaluation are
in [THIRD_PARTY.md](../THIRD_PARTY.md),
[MEASURE_MAPPING.md](MEASURE_MAPPING.md) and
[IMPLEMENTATION_STATUS.md](IMPLEMENTATION_STATUS.md). External project versions
and upstream claims below have not been re-researched in the documentation audit.

## Research conclusion

The strongest starting point for automatic draft generation is **Beat This!**,
particularly the existing **`beat-this` Rust port**. There is no apparent need
to invent a beat/downbeat neural network or introduce Python into the shipped
app just to obtain draft annotations. However, the researched analyzers do not
provide an authoritative editable measure map, guaranteed time-signature
recognition, or a complete correction interface. Those remain product-domain
work.

The important separation is among (1) audio-derived candidate beat/downbeat
events, (2) the user's accepted musical map, and (3) playback scheduling. An
analysis result should be revisable evidence; it should not define permanent
bar numbering by itself. A missed beat or downbeat can otherwise make every
later measure number wrong. Existing professional editing workflows also
preserve manual corrections and explicitly ask for meter hints.

## Useful candidates at a glance

| Candidate | Useful output / reuse | Principal limitation for this project | Research judgment |
| --- | --- | --- | --- |
| Beat This! + `beat-this` Rust | Beats, downbeats, raw logits; native offline inference | No notation-level meter/measure model; imperfect continuity | Best first analysis candidate |
| madmom | Established beat/downbeat processors; inspectable HMM/DBN algorithms | Common decoder chooses one meter for a whole input; bundled weights restrict commercial use | Useful reference/comparison, unattractive default distribution |
| librosa | Onsets, tempo curves, dynamic-programming beat tracking | No downbeat or measure interpretation from `beat_track` | Good baseline and analysis experiments |
| aubio | Small C beat/onset/tempo toolkit, GPL-compatible | Beat tracker API lacks a bar/meter map | Possible lightweight assistance, not a complete draft solution |
| Essentia | Broad C++ analysis; beat times and tempo estimates | AGPL integration implications; separate restrictive model licenses | More machinery than this feature needs |
| QM Vamp + Sonic Visualiser | Existing analysis plugins, annotation, CSV/TSV workflow | Bar tracker explicitly assumes fixed beats per bar | Good independent reference tool and interoperability target |
| rhythm-map | New Rust rhythm-analysis/map SDK | Research-stage inferred clock, not an established editable notation map | Investigate selectively, avoid assuming it solves the product model |

Evidence for each entry follows; these are comparative judgments, not measured
quality rankings on guitar practice recordings.

## Beat This!: algorithm, weights, and limitations

The official ISMIR 2024 work deliberately removes DBN constraints on meter,
tempo range, and abrupt tempo change. Its training includes solo instruments,
classical performances, and pieces with time-signature changes. The authors
report improved beat/downbeat F1 but explicitly acknowledge failures on
difficult or underrepresented material and worse continuity metrics. That
tradeoff is highly relevant: scattered correct detections are less useful for
bar navigation than an uninterrupted correct count. The paper uses a standard
70 ms evaluation tolerance; its headline scores are not evidence of
sample-accurate bar boundaries. [Paper](https://arxiv.org/html/2407.21658v1).

The official package offers `Audio2Beats` for already-decoded audio and
`Audio2Frames` for logits. Main checkpoints are about 78 MB; small checkpoints
about 8.1 MB. Both published model weights **and code** explicitly carry MIT
terms. The authors separately flag copyrighted or limited-Creative-Commons
training material; that caveat should remain in provenance documentation.
Published inference uses PyTorch 2+, NumPy, soxr, einops, and rotary embeddings;
file decoding may additionally need torchaudio/FFmpeg. CPU operation is
supported. The official package therefore works as a reference implementation
without requiring GPU availability. [Official repository and license
statement](https://github.com/CPJKU/beat_this).

The source implements 22,050 Hz mono preprocessing and overlapping
1,500-frame prediction chunks with a six-frame boundary; model frames are
50 Hz. Logits can be retained instead of reducing everything to a timestamp
list. [Inference implementation](https://github.com/CPJKU/beat_this/blob/main/beat_this/inference.py).

Its minimal decoder finds local peaks, thresholds them, deduplicates adjacent
peaks, and snaps downbeats to the closest detected beat. It does **not**
enforce a fixed meter. This permits changing bar lengths, but also permits
missing/extra events and incorrect downbeat grouping. The optional DBN path
reintroduces a `[3,4]` beats-per-bar restriction and 55–215 BPM defaults;
it is unsuitable as an unconditional decoder for variable meter.
[Decoder source](https://github.com/CPJKU/beat_this/blob/main/beat_this/model/postprocessor.py).

Counting the detected beats between downbeats yields an event count, not a
reliable written time signature. It cannot by itself establish the denominator
or distinguish a 6/8 dotted-quarter pulse from a subdivision interpretation.
This is a musical-model inference, not a feature claimed by the library.
Upstream `infer_beat_numbers` counts between downbeats; pickup numbering is
heuristically based on the first full detected measure and warns when there
are insufficient downbeats. It is useful export behavior, not a complete
pickup/numbering policy. [Numbering source](https://github.com/CPJKU/beat_this/blob/main/beat_this/utils.py).

The official changelog records version **1.1.0 on 2026-04-14**, adding raw
activation output, safer checkpoint loading and non-CUDA accelerator support.
[Changelog](https://github.com/CPJKU/beat_this/blob/main/CHANGELOG.md).

## The existing Rust port: especially relevant reuse

[`danigb/beat-this-rs`](https://github.com/danigb/beat-this-rs) exposes a library
and CLI. Its default inference runtime is RTen, with optional ONNX Runtime.
Models are external to the published crate: the repository includes a roughly
270 KB mel graph and 10 MB small ONNX model; the roughly 83 MB main ONNX model
is a download. It accepts decoded mono PCM, avoiding a second decoder in
OhMyLoop, and returns beats, downbeats, mel features and raw logits. The
README reports an Apple M4 MacBook Pro processing a 4:32 track in 4.6 seconds
with Rust versus 5.1 seconds in the Python reference. Those are illustrative
upstream CPU results, not platform guarantees. [README](https://github.com/danigb/beat-this-rs#library-usage).

The manifest specifies **version 1.1.0, Rust 1.89**, RTen 0.24, rubato 3.0,
optional Symphonia 0.6, and optional `ort` 2.0.0-rc.12. Disabling default
features avoids unnecessary CLI/decoder dependencies. OhMyLoop's declared
Rust minimum is 1.88; adopting this version would require an explicit minimum
toolchain decision, although its current local Rust 1.99 can compile that
minimum. Do not assume current dependency versions match the application's
existing versions. [Manifest](https://github.com/danigb/beat-this-rs/blob/main/Cargo.toml).

Version **1.1.0 is dated 2026-10-02**. Its streaming frontend bounds audio
buffering, but retains the mel spectrogram and logits. Upstream main-model
measurements on an M4 Pro report approximately **534 MiB peak for five minutes**
and **620 MiB for an hour**, growing roughly 94 MiB/hour. This is significant
beside OhMyLoop's already-decoded source buffer. The changelog candidly limits
the broader identity/long-input checks: arm64 was exercised; several x86_64
stream/resampler checks and AVX-512 were not. Memory was not measured on Linux.
[Detailed changelog](https://github.com/danigb/beat-this-rs/blob/main/CHANGELOG.md).

The parity test compares **one shared MP3 against Python predictions** at
±70 ms: main-model F1=1.0 and small-model F1≥0.99. It does not compare against
human ground truth, and per-event timing differences are logged rather than
asserted. The main-model test skips when its downloaded model is absent.
Therefore this is useful port-regression evidence, not a proof of broad
musical accuracy or exact timing. [Parity test](https://github.com/danigb/beat-this-rs/blob/main/tests/python_parity.rs).

The checked CI configuration runs Ubuntu, both default RTen and optional ORT;
it is not three-platform evidence. ORT native-library/API version matching is
documented as necessary to avoid initialization hangs. We have not verified
the current CI run results. [CI source](https://github.com/danigb/beat-this-rs/blob/main/.github/workflows/ci.yml).

The port's MIT license retains both JKU and port copyright notices.
The **explicit upstream weights license**, rather than a generic crate license
alone, establishes the model permission. Its export script converts named
official checkpoints into ONNX and records the source download location.
Any shipped conversion should keep those notices and checkpoint/export
provenance; a converted artifact is not a newly trained model.
[Port license](https://github.com/danigb/beat-this-rs/blob/main/LICENSE),
[conversion script](https://github.com/danigb/beat-this-rs/blob/main/scripts/ckpt2onnx.py).

RTen is Rust throughout and currently **CPU-only**, with AVX2/AVX-512, Arm Neon,
and WASM SIMD. It defaults to multiple physical/performance-core threads,
which can be configured. Its MIT/Apache-2.0 licensing and lack of a native
runtime library make it attractive for desktop packaging. Its parallelism
also means analysis must not consume all resources while audio playback is
running. That last sentence is an integration inference.
[RTen project](https://github.com/robertknight/rten).

A separate MIT C++ port exists with ONNX Runtime and minimal/DBN decoding.
It is an alternative reference if runtime or frontend parity becomes a
problem, but brings C++ build/FFI machinery. Its current README includes a
DBN implementation, so older statements that the C++ port omits DBN are no
longer universally correct. [C++ port](https://github.com/mosynthkey/beat_this_cpp).

## Other analyzers: what they actually buy

**madmom.** The RNN downbeat activations and HMM machinery are valuable
research references. A crucial source-level finding: `beats_per_bar=[3,4]`
constructs **separate HMMs**, decodes the same whole activation sequence with
each, then chooses the best-scoring HMM. It does not switch between 3 and 4
within that sequence. Tempo-transition parameters control how strongly it
prefers constant tempo. Other pattern models should not be assumed equivalent
to this standard decoder. [Downbeat processor source](https://github.com/CPJKU/madmom/blob/main/madmom/features/downbeats.py).

madmom source is BSD-2-Clause, but bundled model/data files are
**CC BY-NC-SA 4.0**, and the license specifically includes saved/pickled
processors. An app costing zero is not automatically a noncommercial use,
and such restrictions do not provide unrestricted downstream reuse under
GPL. A sidecar does not remove model-license conditions. Its model-free
algorithms are a different proposition from shipping its trained networks.
[License](https://github.com/CPJKU/madmom/blob/main/LICENSE).

**librosa.** `beat_track` is an onset/tempo/dynamic-programming tracker. The
default estimates global tempo, but an explicitly provided BPM array may
vary per frame. The official dynamic-tempo example demonstrates this
extension, correcting the common claim that librosa necessarily assumes
one tempo. It provides beat locations, not bar/downbeat or time-signature
classification. It is a sensible non-neural baseline and experimentation
tool; shipping it would still add Python/scientific-Python packaging.
[Tracker source](https://github.com/librosa/librosa/blob/main/librosa/beat.py),
[dynamic-tempo example](https://librosa.org/doc/0.11.0/auto_examples/plot_dynamic_beat.html).
Its core license is ISC; dependency notices still require a separate audit.
[License](https://github.com/librosa/librosa/blob/main/LICENSE.md).

**aubio.** C functions expose beat locations, BPM, tempo confidence, silence
thresholds and related controls; this API does not provide bar numbering or
variable time signatures. It could supply onset-snap assistance or a small
classical beat baseline. The project documents Linux/macOS/Windows builds
and GPL-3.0-or-later licensing. No pretrained network is required for its
classical tempo tracker. The GitHub release listing shows 0.4.9; the older
download web page still says 0.4.7, so that page is not a reliable current
version inventory. [Tempo API](https://github.com/aubio/aubio/blob/master/src/tempo/tempo.h),
[project and license](https://github.com/aubio/aubio),
[releases](https://github.com/aubio/aubio/releases).

**Essentia.** `RhythmExtractor2013` returns beat times, global BPM,
intervals and estimates/confidence, with `multifeature` and `degara` methods.
It specifically requires 44.1 kHz audio; Degara's confidence output is always
zero. This particular algorithm is a classical beat/tempo extractor, not
a pretrained model and not a meter mapper.
[Algorithm contract](https://essentia.upf.edu/reference/std_RhythmExtractor2013.html).
The library is AGPLv3; distributed pretrained Essentia models separately
use **CC BY-NC-ND 4.0**. These must not be conflated. Dependencies are optional
in reduced builds but carry their own terms.
[Licensing documentation](https://essentia.upf.edu/licensing_information.html).
GNU documents special GPLv3/AGPLv3 combination permission; AGPL obligations
remain, rather than simply disappearing into GPL. The site's
“non-commercial” description must not be mistaken for a restriction inherent
to standard AGPL, which permits charging for copies. This makes Essentia a
possible but more complicated choice for the current GPL desktop direction.
[GNU compatibility explanation](https://www.gnu.org/licenses/license-compatibility.en.html),
[AGPL text](https://www.gnu.org/licenses/agpl-3.0.html).

**QM Vamp.** The Tempo and Beat Tracker estimates a tempo contour then beat
locations. The Bar and Beat Tracker adds bar starts and counted beats, but
its documentation explicitly says **fixed beats per bar throughout the
input**. It is useful as a conventional independent comparison, or per
known-meter region, not an automatic variable-meter solution.
[Plugin documentation](https://www.vamp-plugins.org/plugin-doc/qm-vamp-plugins.html#qm-barbeattracker).
The SDK is an MIT C/C++ plugin/host interface, not a beat algorithm itself;
plugins have individual licenses. QM plugins are GPL and use QM DSP with
additional notices. [Vamp SDK](https://www.vamp-plugins.org/develop.html),
[QM plugin source/license guidance](https://github.com/c4dm/qm-vamp-plugins).

Sonic Annotator provides batch Vamp extraction, with CSV/RDF and other
writers. Sonic Visualiser provides inspection and annotation. These make
good external truth-inspection/interchange tools. Hosting arbitrary plugins
inside the app adds discovery, native-library/architecture and lifecycle
work that may be unnecessary when one Rust analyzer can be called directly.
The latter is an engineering inference.
[Sonic Annotator](https://github.com/sonic-visualiser/sonic-annotator),
[Sonic Visualiser](https://github.com/sonic-visualiser/sonic-visualiser).

**Newer research.** BeatFM (August 2025) reports improved benchmark results
using MERT/MusicFM foundation models, but still uses DBN postprocessing. The
paper does not establish a drop-in desktop SDK, arbitrary meter changes, or
the model-weight distribution terms needed here. This pass did not locate
and verify a complete licensed deployment artifact. It is worth tracking,
not a reason to delay an available Beat This! evaluation.
[BeatFM paper](https://arxiv.org/html/2508.09790v1).

All-In-One analyzes beats/downbeats and functional song sections using
demixed audio. It requires PyTorch/NATTEN and madmom, and documents a Windows
NATTEN source build. Its repository is MIT, but this pass did not establish
an equally explicit license statement for every downloaded checkpoint and
dependency weight. Its heavier separation/ensemble pipeline is better
considered with future section/stem features than as the immediate measure
map foundation. It also warns of MP3 decoder offsets of tens of milliseconds,
supporting a common decoded-PCM analysis input.
[Official project](https://github.com/mir-aidj/all-in-one),
[repository license](https://github.com/mir-aidj/all-in-one/blob/main/LICENSE).

## Deployment implications for the existing app

Research inference: the native Rust analyzer is the cleanest likely fit.
OhMyLoop already owns source PCM/coordinates; feeding that PCM to analysis
avoids independent MP3 decoder offsets. The analysis result can be converted
to source frames without changing playback audio. Native analysis must run
outside both the device callback and playback-render worker; CPU/thread
budgets, cancellation, progress and memory must be measured with playback
active. A temporary low-rate mono copy or the stream API could limit additional
audio buffering, but the frontend/model still has substantial working memory.

A Python/PyTorch subprocess remains valuable as a reference oracle and
replaceable experimental backend. Tauri officially supports external
binaries, including packaged Python applications, with target-triple-specific
sidecar artifacts. This establishes feasibility, not a small or effortless
PyTorch bundle. [Tauri sidecar documentation](https://v2.tauri.app/develop/sidecar/).

Neither ONNX nor Rust changes the model's statistical limitations. Converting
the graph and substituting resamplers/runtimes requires parity checking on
the same PCM; accelerator support must be distinguished from measured
platform support. RTen's CPU path would avoid shipping GPU runtimes initially.

License conclusion: MIT/ISC/BSD components and Apache-2.0 runtimes can fit a
GPLv3 application while preserving their notices. This is supported by GNU's
license compatibility guidance and Apache's explicit GPLv3 statement. It is
not a completed audit of a resolved future Cargo lockfile.
[GNU license list](https://www.gnu.org/licenses/license-list.html),
[Apache compatibility](https://apache.org/licenses/GPL-compatibility.html).

## Editable musical maps and correction precedents

The following conclusions are product-model inferences to discuss, not a
chosen schema or implementation plan:

- Preserve source-frame anchors separately from musical positions. Speed and
  pitch changes must not move the map on the source recording.
- Distinguish measure boundaries, meter events, pulse/beat-unit information,
  and tempo interpolation. A numerator alone is insufficient for 6/8 or
  additive 7/8; grouping such as 2+2+3 affects accents/count-in behavior.
- A map of bar-start anchors can align measures yet miss expressive timing
  within a measure. Local beat anchors or an explicit interpolation policy
  are needed if beat navigation/click alignment must follow that timing.
- Account for pickups, partial/truncated measures, pre-roll/silence,
  numbering offsets, and unmetered passages instead of forcing the first
  audio sample to be measure 1 beat 1.
- Treat user-confirmed/locked anchors differently from analyzer suggestions.
  Reanalysis must not silently discard accepted corrections. Stable IDs and
  monotonic source order should survive edits and renumbering.
- Distinguish moving a line from rescaling a region, changing its counted
  beats, or shifting all later measures. These edits have different intended
  consequences and should not be hidden behind one ambiguous gesture.

**Logic Pro Smart Tempo is a strong manual-editing precedent**, although its
proprietary implementation is not reusable code. Apple's official support
page says intra-region time-signature changes are only detected when users
supply hints. Its editor distinguishes Beat Hint and Downbeat Hint from
direct exact edits, offers different selected-range adjustment behavior,
and can lock a selected range against reanalysis. These capabilities directly
support a draft-plus-correction workflow rather than blind full automation.
[Smart Tempo support](https://support.apple.com/en-ie/102165),
[manual editing](https://support.apple.com/en-ca/guide/logicpro/lgcp644481aa/mac),
[locking ranges](https://support.apple.com/en-gb/guide/logicpro/lgcp6e8d3a01/mac).

**Ardour offers both editing precedents and inspectable timing code.** Its
tempo ruler supports constant and ramped segments and distinguishes audio
locks from music locks. Grid mode lets users drag bar lines and create ramps;
the time-signature ruler supports changing and complex meters. Separate BBT
position markers can assign a musical origin or reset bar numbering.
[Tempo editing](https://manual.ardour.org/ardours-interface/rulers-and-markers/tempo/),
[Grid mode](https://manual.ardour.org/ardours-interface/the-toolbox/),
[meter editing](https://manual.ardour.org/ardours-interface/rulers-and-markers/time-signature/),
[musical-position markers](https://manual.ardour.org/ardours-interface/rulers-and-markers/barsbeats/).

Its `libtemporal` tempo-map header explicitly separates tempo, meter, and
position markers. Beats advance monotonically even when displayed BBT
positions reset. The header is GPL-2.0-or-later, but references Ardour PBD and
Boost infrastructure. Research inference: this is a valuable semantic and
algorithm reference; extracting it into Rust is unlikely to be as simple as
adding a standalone dependency. Neither integration nor a full dependency
license audit was performed.
[Actual header and dependencies](https://github.com/Ardour/ardour/blob/master/libs/temporal/temporal/tempo.h).

**Rhythm Map is a relevant newer Rust analysis SDK**, currently described as
0.1.0 under Apache-2.0. Its backend-neutral core accepts PCM/observations and
returns versioned beat hypotheses, ambiguity regions, tempo curves,
constant/ramp segments and change points. The optional Beat This! adapter
uses RTen. It is analysis infrastructure rather than an established editable
measure document or meter-change editor.
[Project scope and API](https://github.com/NagareWorks/rhythm-map).

The algorithm documentation explicitly distinguishes detected events from
the musical clock: missing observations need not imply slower tempo, yet
the shipping default remains event-derived. The roadmap records failed
experiments around rubato, omissions and metrical interpretation. Its
half/double-time hypotheses must not be confused with a complete written
time-signature map. Research judgment: evaluate selectively alongside direct
Beat This! output, rather than making its inferred tempo curve authoritative.
Some schema/source pages were unavailable during browsing, so this pass does
not establish the complete implementation or validate its quality claims.
[Algorithm and limits](https://github.com/NagareWorks/rhythm-map/blob/main/docs/ALGORITHM.md),
[recorded research outcomes](https://github.com/NagareWorks/rhythm-map/blob/main/ROADMAP.md).

Ableton supplies another useful precedent: audio-to-musical-position anchors
can make project tempo follow the recording, and its timeline represents
fragmentary bars when a meter change cuts a measure short. This is proprietary
UX evidence, not reusable implementation.
[Tempo leaders and anchors](https://www.ableton.com/en/live-manual/12/audio-clips-tempo-and-warping/),
[fragmentary measures](https://www.ableton.com/en/live-manual/12/arrangement-view/).

## Outstanding evidence and prototype questions

These are research questions, not a work sequence:

1. On representative guitar practice material, how many bars need correction
   with small versus main Beat This! models? Include sparse acoustic intros,
   syncopation, live drums, tempo ramps, abrupt tempo changes, 3/4, compound
   meter, and changing/additive odd meters.
2. Do raw-logit peaks support useful local hints and constrained reanalysis
   near corrected anchors? They are network scores, not calibrated
   probabilities that an entire measure map is correct.
3. What is acceptable source-boundary error for practice navigation, loops
   and later synchronized notation? MIR's ±70 ms is an evaluation convention,
   not our acceptance criterion.
4. What pulse interpretation should be used in compound or additive meter,
   and how should users specify it? Automatic beat counts cannot settle this.
5. Is a bar-anchor-first map sufficient for the initial user experience, or
   must within-bar rubato be editable immediately? Silence/fermatas and
   unmetered passages need explicit semantics.
6. What are runtime/memory, playback-underrun impact and cancelability on
   actual Linux/macOS/Windows targets, particularly older x86 CPUs?
7. What is the artifact/version/checksum/provenance strategy for optional
   model installation, and which dependency notices must accompany builds?

The substantive reuse finding is favorable: a permissively licensed trained
beat/downbeat model already has a native Rust deployment route. The
unresolved work is obtaining a musically reliable, user-owned measure map
from imperfect detections, rather than developing the detector from scratch.
