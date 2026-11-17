# Stem separation research for OhMyLoop

Research date: **2026-10-12**. Scope: separation methods, reusable libraries,
model permissions, desktop deployment and a decision-oriented evaluation plan.
This is a research snapshot, not a feature commitment. No model was downloaded,
no separator installed or built, and no inference, hardware playback or listening
test was performed. Numbers attributed to upstream are author reports. Library
support and model quality below are not locally verified platform results.

The investigation used primary papers, original model publishers, repository
documentation/source and official runtime documentation. Mutable `main` links
describe what was inspected on this date; a pilot must pin commits, model
revisions and checksums. Two metadata-only `hf models info` requests failed on
sandbox network resolution; model metadata was inspected through web browsing.

## Research conclusion

There is enough reusable technology to explore this without training a model.
The most useful first comparison is **HTDemucs six-stem separation for guitar
practice**, **BS-RoFormer-SW six-stem separation as used by feedBack**, a
**modern RoFormer vocal/instrumental model**, and **Banquet for
query-driven guitar extraction**. These answer different questions: instrument
practice, high-quality vocal removal, and finer instrument targeting.

However, **Demucs is not presently a cleared default model for distribution**.
Its code is MIT; its author explicitly excludes weights from that license and
describes them as for scientific/research purposes. A free GPL application is
not automatically research use. Native Rust or ONNX conversions do not change
the original checkpoint's permissions. Treat Demucs as a research comparator
until there is an authoritative grant for the exact weights. [Author's explicit
weight statement](https://github.com/facebookresearch/demucs/issues/327),
[HTDemucs-v4 follow-up](https://github.com/facebookresearch/demucs/issues/508).

For broad experiments, **`audio-separator` is the most practical reusable
adapter** found: one Python API covers several model families. For a shipped
native backend, investigate **`demucs-rs`**, **`demucs.cpp`** or a proven
**ONNX Runtime** conversion after choosing a distributable checkpoint. The
language/runtime decision should follow evidence about guitar usefulness,
artifact permission and actual target-machine performance.

The strongest deployment recommendation is an **optional offline separation
job that produces cached, source-aligned audio**, followed by normal Rust
playback. Separation should not run in the device callback or playback worker.
This extends the existing analysis subprocess boundary rather than changing
native audio ownership. See [ADR 0001](../adr/0001-source-recording-coordinates.md),
[ADR 0002](../adr/0002-native-audio-ownership-and-threading.md),
[ADR 0003](../adr/0003-content-identity-and-preserved-song-documents.md) and
[ADR 0004](../adr/0004-analysis-evidence-and-authored-maps.md).

## What the methods separate

The conventional four-stem task uses vocals, bass, drums and `other`.
[BS-RoFormer paper](https://arxiv.org/html/2309.02612v2).
Research inference: separation estimates signals rather than recovering studio
tracks with certainty. A vocals/instrumental model cannot acquire a guitar head
by relabeling its output; checkpoint target sets must be inspected explicitly.

For OhMyLoop, evaluate at least three distinct benefits:

- **Hear a part:** make guitar notes, articulation and rhythm easier to study.
- **Play along:** attenuate the guitar while preserving useful accompaniment.
- **Balance the band:** control bass, drums, vocals and the remainder independently.

These are product hypotheses. They need different listening criteria: an
isolated guitar can be instructive despite artifacts, while a backing track
can be unpleasant if muting guitar removes cymbals or leaves watery residues.
A class-level guitar stem does not imply independent lead/rhythm guitar tracks.
Two guitars with similar timbre are a substantially harder target than a single
guitar versus drums and vocals; no candidate inspected establishes reliable
player-level separation for arbitrary commercial recordings.

| Method family | Principle | Reusable examples | Practical implication |
| --- | --- | --- | --- |
| Classical spectral/spatial methods | Repetition, harmonic/percussive structure, factorization or stereo cues | HPSS/NMF/Wiener filtering as components | Useful baselines or cleanup; assumptions do not identify arbitrary instruments |
| Spectrogram neural models | Predict masks or target spectra, reconstruct with inverse STFT | Spleeter, Open-Unmix, MDX, BS/MelBand RoFormer | Frontend, phase, windowing and overlap reconstruction are part of the implementation |
| Waveform/hybrid neural models | Estimate waveforms directly or combine waveform and spectral branches | Demucs/HTDemucs | More than a simple matrix model; native parity requires both branches and reconstruction |
| Query-conditioned models | Extract a target described by a class, example audio or text | Banquet, USS, AudioSep, SAM Audio | Target flexibility; prompts and query encoders add dependencies and uncertainty |
| Ensembles | Combine multiple estimates/checkpoints | HTDemucs FT, UVR ensembles | Additional compute/storage; compare against single models before accepting cost |

The spectral, waveform and hybrid distinction is documented in the original
BS-RoFormer paper. Classical-method and integration entries are research
context/inferences, not findings of that paper.
[Paper](https://arxiv.org/html/2309.02612v2).

## Candidates at a glance

The judgments in this table are recommendations for experiments, not a measured
ranking. Evidence and license caveats follow.

| Candidate | Outputs relevant to practice | Reuse route | Recommendation |
| --- | --- | --- | --- |
| HTDemucs / `htdemucs_ft` | Vocals, drums, bass, other | Python API; Rust/C++/ONNX ports | Strong reference; exact original weights have research-use restrictions |
| `htdemucs_6s` | Adds guitar and piano | Same deployment choices | First fixed-stem guitar comparator; experimental piano quality |
| BS-RoFormer-SW | Vocals, drums, bass, guitar, piano, other | `audio-separator`, narrow adapters, ONNX/MLX conversions | High-priority six-stem experiment; original trainer/license unresolved |
| BS-/MelBand RoFormer checkpoints | Commonly vocals/instrumental; checkpoint-dependent additional targets | `audio-separator`, MSST, model-specific Python | Modern quality comparator; verify each checkpoint's owner and terms |
| Spleeter | Two, four or five stems; five adds piano | TensorFlow Python; conversion/ports require separate validation | Explicit permissive pretrained-model statement; useful older baseline |
| Open-Unmix | Four stems | PyTorch model/library | Inspectable reference; default `umxl` weights are noncommercial |
| MDX / MDX23C / UVR models | Mostly vocals/instrumental, some multistem | ONNX for selected MDX models, Python for others | Potential smaller native path; target/license/frontend vary by artifact |
| Banquet / query-bandit | Audio-query instrument extraction, including guitar subclasses | Research PyTorch inference | Particularly relevant guitar experiment; checkpoint/encoder license audit incomplete |
| AudioSep / USS | Text or class-conditioned general sounds/instruments | Research PyTorch inference | Exploratory; not established full-band stereo practice replacements |
| SAM Audio | Text/span/visual target and residual | Large PyTorch ecosystem, gated checkpoints | Track research; custom restrictive license and deployment cost |
| SCNet / Windowed RoFormer | Efficient spectral separation variants | Python reference implementations | Efficiency watchlist; vocal/general four-stem results do not prove guitar separation |

## Demucs: useful baseline, important maintenance and license limits

Meta's original repository was archived on **2025-01-01**. The original author
points to `adefossez/demucs`, explicitly describing the project as no longer
actively maintained, with important bug fixes only. HTDemucs combines waveform
and spectrogram paths through cross-domain transformers. `htdemucs_6s` adds
guitar and piano; upstream calls it experimental and warns about piano bleeding
and artifacts. `htdemucs_ft` is a four-model fine-tuned bag, not a guitar model.
The sparse research variant described in the paper was not released as an
ordinary portable checkpoint. [Original project and model descriptions](https://github.com/facebookresearch/demucs).

The weight restriction is directly documented by the original author, not
inferred solely from training-data licenses. In issue #327 the author says the
weights are outside MIT and only for scientific purposes; in issue #508, which
specifically asks about HTDemucs v4 in commercial products, the author again
states research purpose. There was no superseding unrestricted grant in the
sources inspected. This is a distribution gate, not a claim that every possible
research experiment or separately trained Demucs checkpoint is forbidden.
[Issue #327](https://github.com/facebookresearch/demucs/issues/327),
[Issue #508](https://github.com/facebookresearch/demucs/issues/508).

The successor API provides `Separator.separate_tensor`, so a reference harness
can pass the application's decoded PCM rather than decode an MP3 independently.
It exposes model, segment, shifts, overlap, device, jobs and chunk callbacks.
The device can differ from the whole-track storage device. This is relevant to
progress and memory management, but API callbacks alone do not establish
immediate cancellation of native inference. [Actual API documentation](https://github.com/adefossez/demucs/blob/main/docs/api.md).

Upstream chunking and overlap are part of the output: `apply_model` handles
padding, center trimming and weighted accumulation. A native port must preserve
these details and normalization as well as the network. Random shift averaging
must be seeded or disabled for reproducible comparisons. [Inference source](https://github.com/adefossez/demucs/blob/main/demucs/apply.py).

The CLI can rescale stems individually to prevent clipping, which changes their
relative gains. For playback mixing experiments, use unnormalized float PCM and
explicit headroom policy rather than assume CLI WAVs reproduce the original
mix. Two-stem mode computes the model's full source set first; it does not
automatically make inference cheaper. [CLI source](https://github.com/adefossez/demucs/blob/main/demucs/separate.py).

## Modern spectral models and reusable adapters

**BS-RoFormer** splits the complex spectrum into bands, alternates temporal
and band attention, and uses rotary positional embeddings to estimate complex
masks. The paper reports SDX23 success and MUSDB18HQ experiments; these are
four-stem results, not guitar-specific evidence. Headline numbers from the
paper's trained systems cannot be assigned to an arbitrary community checkpoint.
[Original paper](https://arxiv.org/html/2309.02612v2).

**Mel-Band RoFormer** uses overlapping mel-based band projection and reports
improvements over BS-RoFormer on vocals, drums and other in its MUSDB18HQ
experiments. This is an architecture claim under the paper's conditions.
[Original paper](https://arxiv.org/abs/2310.01809).

`lucidrains/BS-RoFormer` is an MIT PyTorch implementation supporting stereo and
multiple stems. It supplies architecture code; it should not be treated as an
authoritative license for independently trained checkpoints. Its current
dependency surface includes PyTorch and several attention/tensor helper
packages. [Implementation](https://github.com/lucidrains/BS-RoFormer),
[Manifest](https://github.com/lucidrains/BS-RoFormer/blob/main/pyproject.toml).

**Music-Source-Separation-Training (MSST)** is MIT research/training code with
inference for MDX23C, HTDemucs, BS-RoFormer, MelBand RoFormer and other families.
Its pretrained-model catalogue supplies architecture/config/checkpoint pairs.
Some listed scores are measured on MVSep's Multisong dataset, while filenames
can contain a different training/validation SDR. A filename such as
`...sdr_12.9755.ckpt` is neither a portable quality guarantee nor the catalogue's
evaluation score. [Repository](https://github.com/ZFTurbo/Music-Source-Separation-Training),
[Catalogue and metric notes](https://github.com/ZFTurbo/Music-Source-Separation-Training/blob/main/docs/pretrained_models.md).

**Kimberley Jensen's vocal MelBand model** is a particularly concrete pilot
candidate: the author's repository links directly to the original model host,
documents vocal/instrumental output and eight-second training chunks, and
explains the overlap/runtime tradeoff. The original host currently labels the
model **MIT**, although the model card's prose is empty. Preserve that original
revision and license metadata; this grant does not automatically apply to later
fine-tunes or similarly named mirrors. [Author's inference repository](https://github.com/KimberleyJensen/Mel-Band-Roformer-Vocal-Model),
[Original model publisher](https://huggingface.co/KimberleyJSN/melbandroformer).

**UVR** is a desktop application and model ecosystem, not one separation
algorithm. Its GUI code is MIT; the authors explicitly ask third-party users
of their models to comply with MIT and retain UVR credit. That is meaningful
evidence for UVR-authored models, but not an automatic grant for Demucs or every
external checkpoint exposed by the GUI. [UVR license and model attribution](https://github.com/Anjok07/ultimatevocalremovergui#license).

**`audio-separator`** exposes UVR-related MDX, VR, Demucs and MDXC/RoFormer
families through a CLI and Python package. This is the best broad experiment
adapter found. The source downloads model files/configs from several upstream
and fallback locations, including optional VIP entries. For a reproducible
OhMyLoop experiment, allowlist exact public model/config pairs and supply a
controlled local model directory; do not rely on a mutable registry/default.
[Adapter](https://github.com/nomadkaraoke/python-audio-separator),
[Download and registry implementation](https://github.com/nomadkaraoke/python-audio-separator/blob/main/audio_separator/separator/separator.py).

The inspected adapter README reports CPU, CUDA, MPS/CoreML and experimental
DirectML paths. Its DirectML introductory paragraph says the path is not tested
in CI, while its table says selected architectures are CI-tested on a T4 in
WDDM mode. It also describes RoFormer CPU fallback due to allocator problems
and unsupported Demucs LSTM on DirectML. This internal documentation inconsistency
means AMD/Intel support must remain unverified here. The inspected Apple path
requires a recent PyTorch/macOS baseline and can fall back for complex spectral
operations; avoid copying package versions without checking a pinned release.
[Current adapter deployment documentation](https://raw.githubusercontent.com/nomadkaraoke/python-audio-separator/main/README.md).

**MDX is not interchangeable with MDX23C.** Selected UVR MDX artifacts are ONNX
graphs; RoFormer/MDXC checkpoints usually pair PyTorch weights with YAML
configuration. ONNX portability of one MDX model is not evidence that every
model in the adapter exports or executes on every provider. The registry/source
should define input frequency dimensions, chunk length, target and frontend.
[Registry implementation](https://github.com/nomadkaraoke/python-audio-separator/blob/main/audio_separator/separator/separator.py).

**New dedicated adapters** `openmirlab/bs-roformer-infer` and
`melband-roformer-infer` provide narrower checkpoint-management/inference paths.
The BS adapter itself documents vanished upstream model URLs and fallback-host
audits. This is useful first-party evidence about its own registry maintenance,
not independent confirmation of the model owners' license claims. Their value
is simpler experimentation; evaluate pinning and output parity before depending
on them. [BS adapter](https://github.com/openmirlab/bs-roformer-infer),
[MelBand adapter](https://github.com/openmirlab/melband-roformer-infer).

### BS-RoFormer-SW: the six-stem feedBack candidate

This specific checkpoint deserves prominence over vocal-only RoFormer models
for guitar practice. The adapter registry calls it **BS Roformer SW by jarredou**,
with `BS-Roformer-SW.ckpt` and `BS-Roformer-SW.yaml`; feedBack uses the alias
`bs_roformer_sw`. Do not equate this name with Smule's **Windowed RoFormer**:
they are distinct artifacts with different outputs and provenance.
[Adapter model registry](https://github.com/nomadkaraoke/python-audio-separator/blob/main/audio_separator/models.json),
[feedBack splitter](https://github.com/got-feedBack/feedBack-plugin-stem-splitter).

A converter's first-party model card documents six outputs in the order bass,
drums, other, vocals, guitar, piano; 44.1 kHz stereo; an original approximately
699 MB checkpoint; and source SHA-256
`24e7d35ee9c64415673d3fd33e06a67cac2c103c5df6267ba1576459c775916e`.
It explicitly labels the license **unknown**, says the original trainer is
unknown, and describes its work as a format conversion. This is the converter's
provenance report, not a verified grant from the original trainer. The vanished
jarredou host and available mirrors do not settle redistribution permission.
[Conversion card with source identity and license statement](https://huggingface.co/lumabeat/bs-roformer-sw).

`elicwhite/bs-roformer-web` provides a six-stem ONNX/WebGPU implementation and
export/frontend work worth reviewing for reuse. It reports approximately
336/669 MB FP16/FP32 graphs and an M-series example of 159 seconds of audio
processed in 63 seconds. These are upstream illustrative results, not our
benchmark or an established fallback CPU path. Its MIT implementation license
cannot supply the missing original checkpoint grant.
[ONNX/browser implementation](https://github.com/elicwhite/bs-roformer-web).

Research judgment: include SW in a tightly scoped research comparison after
checking permitted use, but do not select it as a shipped default based on
feedBack's adoption. Obtaining authoritative provenance and a grant is an early
gate. The six-stem output is well matched to the product question; licensing
and measured guitar quality remain separate unanswered questions.

## feedBack case study supplied by the user

The active stem work lives in separate plugins/server repositories rather than
the main repository's removed splitter stub. The splitter supports managed
local server, Docker, remote service and local Python execution, exposes model
download controls, preserves the full original mix and uses canonical six-stem
labels. Its default audio-separator model is BS-RoFormer-SW; Demucs 6s is an
alternative. Its managed setup/peak-memory estimates are upstream claims, not
local measurements. [Splitter README](https://github.com/got-feedBack/feedBack-plugin-stem-splitter/blob/acf8dfecfeb19562a6a296c1576a2b1ac3050ed3/README.md).

The orchestration is a useful reference for progress, setup and provenance, with
an important cancellation distinction: its local Demucs subprocess is polled
and terminated/killed on cancellation, while its in-process audio-separator
call does not receive a cancellation callback. Its provenance records engine,
model and schema, but does not establish exact checkpoint hashes or source
sample alignment. It converts output through FFmpeg to OGG for its package.
[Reviewed splitter source](https://github.com/got-feedBack/feedBack-plugin-stem-splitter/blob/acf8dfecfeb19562a6a296c1576a2b1ac3050ed3/split_stems.py).

The server maps the SW alias to the checkpoint, caches uploaded inputs using a
truncated content hash plus model slug, and its URL route keys on URL text.
Those keys do not include a complete model/config/preprocessing identity.
This reinforces the recommendation to use OhMyLoop's complete source identity
and versioned derivative manifest. The server's RoFormer child is a thin lazy
`audio-separator` invocation, not a separate algorithm.
[Pinned server source](https://github.com/got-feedBack/feedBack-demucs-server/blob/115fb80ee927bad300b35bc7cf7cd1fb16505426/server.py),
[RoFormer runner](https://github.com/got-feedBack/feedBack-demucs-server/blob/115fb80ee927bad300b35bc7cf7cd1fb16505426/run_roformer.py).

The reviewed splitter and server declare **AGPL-3.0-only**. They are valuable
architecture references; copying/linking their implementation requires a
deliberate licensing decision. GNU describes GPLv3/AGPLv3 combination rules,
with AGPL obligations retained; this is not equivalent to adding MIT library
code to the present GPL application.
[Splitter license](https://github.com/got-feedBack/feedBack-plugin-stem-splitter),
[Server license](https://github.com/got-feedBack/feedBack-demucs-server),
[GNU compatibility guidance](https://www.gnu.org/licenses/license-compatibility.en.html).

## Older baselines with clearer roles

**Spleeter** is TensorFlow/Python with pretrained two-stem
vocals/accompaniment, four-stem VDBO and five-stem VDBO+piano models; it has no
guitar-specific head. The README warns about Apple M1/TensorFlow compatibility.
Its GPU speed claim is historical and not a present desktop measurement.
[Project](https://github.com/deezer/spleeter).

Unlike a code-only license badge, the authors' paper explicitly states that
**source code and pretrained models are MIT**. This makes Spleeter a useful
permission-oriented baseline, even if a pilot ultimately finds its outputs
less suitable for guitar practice. Its authors describe buffering, so faster
than real-time throughput is not evidence of low-latency live separation.
[Author paper in the repository](https://github.com/deezer/spleeter/blob/master/paper.md).

**Open-Unmix** is an inspectable PyTorch reference: per-target bidirectional
LSTM magnitude-mask models, followed by generalized multichannel Wiener
filtering and inverse STFT. It targets vocals, drums, bass and other. Its
default **`umxl` weights are explicitly CC BY-NC-SA 4.0**, whereas the source
code is MIT. Do not choose the default by assuming the package's code license
covers every model. For `umx`/`umxhq`, inspect the exact hosted artifact terms
before distribution; this pass did not finish that artifact audit.
[Official implementation and model/license descriptions](https://github.com/sigsep/open-unmix-pytorch).

## Guitar-focused and open-vocabulary research

**Banquet (`query-bandit`) deserves a separate guitar experiment.** The
authors report that its 24.9-million-trainable-parameter query model outperformed
six-stem HTDemucs on guitar and piano in their MoisesDB evaluation, and supports
narrow classes such as clean acoustic guitar. These are paper results, not
OhMyLoop recordings or proof that lead and rhythm players can be separated.
The query is example audio encoded with a music-recognition PaSST model, not
simply a natural-language instrument name. [Original paper](https://arxiv.org/abs/2406.18747).

The MIT repository exposes `inference_byoq` with an input file and separate
query file, links Zenodo checkpoints, and recommends `ev-pre-aug.ckpt`.
Its example's batch size usually fits an RTX 4090; that is not a CPU or Apple
Silicon performance result. The linked Zenodo record could not be retrieved
during this pass, so checkpoint license, sizes and query-encoder terms remain
unverified. [Author's code and exact invocation](https://github.com/kwatcharasupat/query-bandit),
[Checkpoint record to audit](https://zenodo.org/records/13694558).

Research inference: an exemplar query could help target acoustic versus
distorted guitar or a rarer instrument. The first experiment should use a clean
external example and a short isolated passage from the same song, where
available, then compare extraction failures and absent-target behavior.
Selecting a passage is not sufficient if it still contains the whole band.
Do not promise specificity until those cases are tested.

**AudioSep** is an MIT-code language-query separation implementation. It
documents 32 kHz processing, CPU/CUDA selection and optional chunk inference.
Its original benchmark includes instrument separation among general audio
tasks, but it is not the same evaluation as 44.1 kHz stereo four-stem music
separation. The exact separation and CLAP/query-encoder checkpoint permissions
were not established here. Treat it as a prompt experiment, not the default
music engine. [Official AudioSep repository](https://github.com/Audio-AGI/AudioSep).

**ByteDance USS** is a general class-conditioned research implementation
combining a tagging/query network and conditional separator. It is useful for
seeing how instrument/event class conditioning works. Its weak-label general
sound task and dependencies differ from a polished music demixer; a complete
weight-license and desktop deployment audit was not performed.
[Official code](https://github.com/bytedance/uss),
[Original paper](https://arxiv.org/abs/2305.07447).

**SAM Audio** offers text, temporal-span and visual prompts and produces target
and residual signals. Its setup recommends a CUDA-compatible GPU, and its
checkpoints require requesting access and authentication. Its diffusion/flow
matching foundation-model approach is interesting for user-directed extraction,
but does not establish a small native music-separation library.
[Official repository](https://github.com/facebookresearch/sam-audio),
[Original paper](https://arxiv.org/abs/2512.18099).

SAM Audio's **custom SAM License covers code and trained weights**, requires
redistribution under its agreement and includes restrictions beyond ordinary
MIT/Apache terms, including reverse-engineering restrictions. It should not be
described as a GPL-compatible component solely because source/checkpoints are
available. It would need a separate distribution/licensing decision; downloading
it as a sidecar does not erase the terms.
[Actual SAM Audio license](https://github.com/facebookresearch/sam-audio/blob/main/LICENSE).

## Efficiency watchlist

**SCNet** is an MIT-code sparse-compression separation implementation with
published small/large checkpoints trained on MUSDB. It is worth considering
as an efficiency comparator for general four-stem separation. The repository's
checkpoint download links and code license alone do not complete an artifact
redistribution audit. [Official implementation](https://github.com/starrytong/SCNet).

**Windowed RoFormer** replaces full temporal attention with windowed sink
attention. The authors report recovering 92% of the original vocal model's SDR
performance while reducing FLOPs 44.5-fold. FLOPs reduction is not equivalent
to end-to-end speedup, and vocal SDR does not evaluate guitar removal. The
official code is MIT and the official hosted checkpoint is marked MIT; its
listed checkpoint size is about 914 MB, which should not be confused with peak
working memory. [Official code](https://github.com/smulelabs/windowed-roformer),
[Official model files/license metadata](https://huggingface.co/smulelabs/windowed-roformer/tree/main),
[Paper](https://arxiv.org/abs/2510.25745).

## Native and ONNX deployment routes

| Route | Verified upstream description | Principal unresolved evidence |
| --- | --- | --- |
| `nikhilunni/demucs-rs` | Native Rust HTDemucs, Burn/wgpu; Metal macOS, Vulkan Linux/Windows; standard/6s/FT; Apache-2.0 code | Exact checkpoint permission, numerical parity, stable library surface, GPU availability and target measurements |
| `sevagh/demucs.cpp` | C++17 hybrid/transformer Demucs; Eigen/OpenMP computation with ggml-format weights; MIT code | Build/FFI packaging, parity and CPU throughput; original weight restrictions |
| `StemSplit/demucs-onnx` | HTDemucs/6s/FT export and NumPy+ORT inference; MIT code | Exact graph/provider parity, native frontend reuse, original weight restrictions |
| `0xShug0/audio.cpp` | Broader ggml C++ audio engine with separation models | Large dependency surface and model-specific maturity; checkpoint provenance and runtime evidence |
| Python/PyTorch sidecar | Most direct reference for community checkpoints | Bundle size, signing, accelerator dependency matrix and unsupported operators |

The table describes available work, not verified packages suitable for shipping.
Sources: [Rust port](https://github.com/nikhilunni/demucs-rs),
[C++ port](https://github.com/sevagh/demucs.cpp),
[ONNX conversion](https://github.com/StemSplit/demucs-onnx),
[audio.cpp](https://github.com/0xShug0/audio.cpp).

`demucs-rs` has a separate `demucs-core` inference crate and Symphonia-based CLI,
which is a promising architectural fit. The README requires a suitable GPU;
do not infer a supported CPU fallback because Burn has other backends. The
listed converted artifacts are about 84 MB for standard/6s and 333 MB for FT.
These are download/storage figures, not total GPU memory. The port's Apache
license applies to its implementation, not a new license for original weights.
[Port architecture and prerequisites](https://raw.githubusercontent.com/nikhilunni/demucs-rs/main/README.md).

`demucs.cpp` is a useful CPU-oriented reference. It uses Eigen and OpenMP for
inference; ggml in the description also concerns weight serialization. Do not
assume this means modern ggml Metal/CUDA acceleration throughout its layers.
Its low-memory variant trades performance for memory. Any Rust integration
would require a small explicit FFI or an isolated executable.
[Actual port README](https://raw.githubusercontent.com/sevagh/demucs.cpp/main/README.md).

`demucs-onnx` documents rewriting otherwise unsupported export pieces, including
spectral operations and attention. It is a real conversion project, not merely
instructions to call `torch.onnx.export`. Nonetheless its production/parity
claims are upstream reports; no exported graph or test result was exercised
here. The reusable contribution could be graph conversion and frontend logic,
with a separate native inference wrapper.
[Converter implementation overview](https://raw.githubusercontent.com/StemSplit/demucs-onnx/main/README.md).

ONNX Runtime provides CPU and multiple execution providers, including CUDA,
CoreML and DirectML. Provider availability does not guarantee an entire model
runs on that accelerator: graph support, partitioning, fallback and package
configuration matter. Test the exact graph on each target and record unsupported
nodes/device transfers. Rust bindings are an integration mechanism, not a
separator or evidence of model conversion correctness.
[Official execution-provider documentation](https://onnxruntime.ai/docs/execution-providers/).

OhMyLoop already has a Rust/RTen deployment precedent for Beat This!, documented
in [the earlier analysis research](BEAT_MAPPING_RESEARCH.md). That is evidence
for one graph, not HTDemucs or RoFormer operator compatibility. Research
inference: compare reuse of RTen with Rust `ort` bindings to ONNX Runtime before
introducing a new engine. Candle or tract are alternative general tensor/graph
runtimes, not stem-separation libraries; this pass did not establish a validated
port/checkpoint/frontend combination using them. Any candidate needs explicit
operator coverage, spectral/complex representation and reconstruction parity.

Research inference: **CPU should be the dependable compatibility baseline if
we ship this feature**, with optional accelerators after measured benefit.
CUDA adds NVIDIA-specific runtime requirements; Metal/MPS/CoreML are distinct
paths, and DirectML support must be verified per architecture. A GPU-only native
backend is attractive as an experiment but leaves older/integrated-device
users without a known fallback. Avoid promising acceleration from a detected
provider alone.

Tauri supports packaging external binaries with target-triple-specific sidecar
artifacts. This makes a prototype Python/native executable boundary feasible;
it does not establish that a PyTorch bundle is small or effortless to sign,
notarize and maintain. [Official Tauri sidecar documentation](https://v2.tauri.app/develop/sidecar/).

## License and artifact decisions

This is evidence collection, not a completed legal audit. Before shipping,
record separate entries for implementation code, runtime/dependencies,
checkpoint weights, configuration, converted derivatives and attribution.
An exact grant from the original trainer carries more weight than a license
badge on a mirror or conversion repository.

| Artifact | Evidence found | Current disposition |
| --- | --- | --- |
| Original Demucs/HTDemucs weights | Author explicitly scientific/research purpose; outside MIT | Research comparator; require new grant or separately cleared training |
| Spleeter pretrained models | Authors' paper explicitly includes models in MIT grant | Strong permission baseline; retain notices and exact artifact identity |
| Open-Unmix `umxl` | Explicit CC BY-NC-SA 4.0 | Do not present as unrestricted GPL redistribution |
| Original Kimberley MelBand vocal model | Original host says MIT; directly linked by author | Good pilot candidate; archive grant/metadata and audit dependencies |
| UVR-authored models | Authors request MIT compliance for their models | Audit exact owner/model; external registry entries retain own terms |
| ViperX / SW / later RoFormer fine-tunes | Download/config paths found; no complete original-trainer license audit | Evaluate only after artifact-specific permission checks; no blanket RoFormer license |
| Banquet separation and query-encoder weights | Code MIT; checkpoint link exists; hosted terms retrieval failed | Promising guitar pilot; audit both weight sets before distributing |
| SAM Audio | Custom license applies to materials including weights and code | Separate restricted-license decision; unattractive default for GPL desktop |

The evidence links are in the candidate sections above. Converting a `.ckpt`
to ONNX, GGUF, Burn or another serialization does not create a fresh permissive
grant. Making a model an optional download or subprocess is a packaging choice,
not proof that its restrictions cease to apply. Training-data permissions are
another provenance axis; do not assert that every model inherits its dataset's
license automatically, or that MIT code establishes training-data clearance.

For ordinary MIT and Apache-2.0 implementation components, GNU and Apache
document compatibility with GPLv3 when their conditions/notices are preserved.
This does not resolve custom, research-only or noncommercial model conditions.
[GNU license list](https://www.gnu.org/licenses/license-list.html),
[Apache GPL compatibility](https://apache.org/licenses/GPL-compatibility.html).

## Architecture implications for OhMyLoop

These are proposals inferred from the research and existing ADRs, not a selected
schema or implemented feature. They preserve the CLI prototype and current
practice workflow; researching stems does not expand the implementation milestone.

1. **Separate once, play many times.** Run a cancellable offline job in a child
   process with bounded inference threads/device use. Expose status/progress
   and retain ordinary playback during a job only if resource tests permit it.
   Publish completed results atomically; never replace a good cache with partial
   output from cancellation, out-of-memory or process failure.
2. **Use the original decoded recording as input authority.** Prepare PCM through
   the app's decoding path outside playback processing. Keep input channel/rate,
   resampling, model padding, crop and output-length rules explicit. Validate
   output against original source-frame coordinates before accepting it.
3. **Keep derivatives separate from authored documents.** Key a stem manifest by
   source SHA-256, model/config/checkpoint identity, preprocessing implementation,
   settings, seed and inference/backend version when numerically relevant.
   Retain the source recording and user notes/maps/sections. Cache eviction must
   not erase practice annotations. This follows ADRs 0001 and 0003.
4. **Validate at the result boundary.** Check finite PCM, bounded channel/stem
   count, declared sample rate, expected frame count, stem identities, hashes,
   delay/crop metadata and safe output paths. Handle unsupported future manifest
   versions conservatively. Reject stale source/model results explicitly.
5. **Treat model acquisition separately.** Pin publisher revision, exact bytes,
   checksum, configuration and license/provenance. Keep optional model packages
   and audio caches outside source control, with reproducible conversion scripts.
   Runtime must not execute arbitrary checkpoint-provided code just to infer.
6. **Keep one source transport.** All stem cursors share seek/loop generations
   and source-frame anchors. Speed and pitch do not move saved sections or the
   measure map. The callback consumes preallocated rendered audio as today;
   separator work and cache I/O stay outside it. This follows ADRs 0001/0002.
7. **Choose a mixing/DSP policy deliberately.** Mixing cached stems before one
   Rubber Band instance minimizes processing cost but gain changes then wait
   for already-rendered audio. Per-stem stretchers allow more independent control
   but multiply DSP/state and require phase/time alignment. Test both behavior
   and practical response time before selecting a design.
8. **Keep original playback available.** A user may compare original, isolated
   target and accompaniment. If estimates do not sum to the original, define a
   residual explicitly; never silently normalize each stem and call their sum
   the unchanged recording.

Storage budget is material even when models are small. **Calculated example:**
five minutes of six stereo stems at 44.1 kHz float32 requires
`300 × 44,100 × 2 × 4 × 6 = 635,040,000` bytes, about **606 MiB**, before
the original, intermediates or inference memory. Four such stems need about
404 MiB. Lossless compression is signal-dependent; no fixed saving is assumed.
Keeping complete original and all stems in RAM could substantially increase
the existing decoded-buffer footprint. Measure disk-backed playback/cache
options without moving file access into the device callback.

Cropping only a practice section reduces work, but boundary context changes
the estimate. For an excerpt pilot, include context around the requested span,
then crop using declared source coordinates; compare against whole-track
inference. Padding is necessary but not proof of unchanged quality near edges.
Overlapping chunk reconstruction must be tested for seams and transient loss.

## Evaluation and pilot plan

The first pilot should answer **whether the output improves practice**, then
whether we can deploy it sustainably. It should remain a standalone harness
and reproducible report until those questions are answered.

### Stage A: permission and reproducibility gate

- Select exact checkpoints/configs and archive original grants, notices,
  publisher URLs, revisions, checksums and conversion commands.
- Start with HTDemucs 6s as a research-only guitar reference and BS-RoFormer-SW
  as the six-stem candidate from feedBack after checking research-use permission;
  obtain an authoritative SW distribution grant before any shipping decision.
  Include original Kimberley
  MelBand vocals as a modern vocal comparator, and Spleeter as a permissive
  baseline. Add Banquet after validating separation/query-encoder artifact terms.
- Pin the Python reference and use common decoded PCM, float outputs and fixed
  normalization. Disable or seed random shifts. Record chunk/overlap/precision.
- Use a controlled model directory; no heavyweight download or auto-install is
  implied by this document. Keep the harness outside playback and permanent UI.

### Stage B: usefulness and correctness

Build a small rights-cleared corpus covering clean/acoustic and distorted guitar,
lead and rhythm together, solos, sparse intros, reverb/delay, cymbal-heavy mixes,
live/bleeding recordings, mono/stereo, silent sections and absent target.
Include entire tracks and short loop/excerpt cases. Have paired original stems
where possible; commercially mastered mixtures alone provide no ground truth.

MUSDB18's conventional labels are vocals/drums/bass/other and thus cannot
directly score a guitar head. Its access/use conditions restrict the benchmark
recordings to academic purposes. MoisesDB offers finer instrument stems but is
CC BY-NC-SA 4.0. Neither dataset is a blanket grant to put its audio into GPL
tests or releases. Prefer self-recorded or explicitly cleared multitracks for
committed fixtures, plus generated impulses/silence/tones for alignment tests.
[MUSDB18 documentation](https://sigsep.github.io/datasets/musdb.html),
[MoisesDB licensing](https://github.com/moises-ai/moises-db#licensing).

Measure objective error only with correctly aligned ground truth. `museval`
implements BSSEval v4, including distortion-filter matching and configurable
aggregation; its score is not identical to plain waveform SDR or SI-SDR. Record
metric variant, aggregation, silent-target policy and any allowed alignment.
Do not place community filename SDRs, MUSDB medians and private Multisong scores
in one ranking as if they shared an evaluation protocol.
[Official museval implementation and metric explanation](https://github.com/sigsep/sigsep-mus-eval).

Recommended pilot observations:

- Target retention and bleed; pick attack, bend/vibrato and rhythm intelligibility.
- Backing-track damage when target is muted; missing cymbals, keys or vocal harmonics.
- Hallucinated target in target-absent passages and silence/noise behavior.
- Stereo image, phase behavior, gain preservation and sum/residual error.
- Exact output length, start/end alignment and transient offsets after resampling.
- Chunk seams, excerpts versus whole-track results, and loops across seams.
- Listening after the app's normal speed/pitch transformations, especially slow
  practice where artifacts may become more distracting.

Listening should be blinded where practical and recorded separately from metrics.
An aggregate four-stem SDR increase is not sufficient to establish improved
guitar practice. Do not use delay-optimizing metrics as evidence of exact source
alignment; verify transport alignment separately with known signals.

### Stage C: runtime and portability

Measure wall time and real-time factor, cold/warm model load, peak resident memory,
peak GPU memory, disk use, cancellation latency, progress granularity, numerical
parity and failure recovery. Repeat with playback, seeking and loops active;
record CPAL underruns/device behavior independently from separator throughput.

| Target | Baseline to establish | Optional accelerator experiment |
| --- | --- | --- |
| macOS Apple Silicon | CPU correctness and memory; signed offline packaging | MPS/CoreML or Burn Metal on exact model |
| Linux x86_64 | CPU correctness on realistic older hardware | CUDA and/or Vulkan; actual driver/operator checks |
| Windows x86_64 | CPU correctness and sidecar/runtime packaging | CUDA; selected ONNX DirectML graph, AMD/Intel separately |
| macOS Intel / other supported architectures | Explicit applicability decision and CPU evidence | Only if there is a concrete supported path |

A configured CI job is not a platform pass. Automated alignment/parity checks,
real device-playback checks and subjective listening results should be reported
as distinct evidence, consistent with AGENTS.md. Native conversion should only
be considered successful after parity on representative PCM and musical listening
at the shipped settings, not merely successful model loading.

### Decision gates

There are no invented universal performance thresholds in this research. Agree
acceptable hardware, wait time, storage/memory and musical failure rate before
implementation. A pilot should explicitly resolve:

1. Is fixed six-stem guitar isolation/removal useful enough, or is fine-grained
   guitar querying essential?
2. Is vocal removal independently valuable even if guitar extraction remains weak?
3. Which exact weights have distribution terms consistent with a free GPL product?
4. Can the fallback CPU route complete a practical song on representative machines?
5. Does inference coexist with playback, or should the app pause/throttle the job?
6. Which native port preserves reference quality and reduces packaging cost?
7. Are mixer controls sufficient, or do gains need faster response than one mixed
   Rubber Band pipeline provides?

## Recommended shortlist and follow-up

**Experiment adapter:** pin `audio-separator` for breadth, with direct model
references as oracles when adapter behavior differs. **Guitar comparator:**
HTDemucs 6s under research terms and BS-RoFormer-SW after resolving its
provenance/use terms, alongside Banquet after artifact audit.
**Modern vocal candidate:** original Kimberley MelBand model with original MIT
metadata preserved. **Permission baseline:** Spleeter pretrained models.
**Native candidates:** `demucs-rs`, `demucs.cpp` and `demucs-onnx`, selected by
parity/resource evidence and cleared model artifacts.

Do not start by writing a new neural separator or making an unverified model
the app's permanent default. The unresolved question is the combination of
musical usefulness, checkpoint permission and dependable desktop deployment.
This research finds viable reuse paths, but it does not settle guitar quality,
offline performance, model redistribution or synchronized multistem playback.
