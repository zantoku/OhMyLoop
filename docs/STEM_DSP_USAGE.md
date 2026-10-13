# Stem DSP listening harness

The terminal player uses the native practice engine: aligned source blocks are
mixed before one Rubber Band R3 processor. It does not attach stems to library
songs or change desktop playback. Shared DSP acceptance still requires the
maintainer's listening approval.

Run from the repository root, substituting the full reference recording path:

```sh
CARGO_HOME="$PWD/.cargo-home" cargo run --locked --release --example stem_player -- \
  /path/to/BONES-reference.wav \
  artifacts/stem-prototype/bones-demucs-6s \
  artifacts/stem-prototype/bones-roformer-sw
```

The reference digest must match the provenance in both completed prototype
manifests. Equal duration does not establish onset alignment. Original audio,
stems and manifests are read without modifying them.

Before playback, the player prepares private temporary, seekable PCM and scans
all selected sources. It reports elapsed preparation time, temporary disk size
and one common attenuation in dB. Preparation uses bounded buffers; temporary
disk space scales with duration. Normal exit and handled failure remove prepared
files. Abrupt termination has no recovery or automatic cleanup workflow.

Use `--check` to validate and prepare without opening an audio device. Use
`--smoke` for a short scripted hardware exercise; that is not a listening verdict.
`--help` prints the authoritative control help.

The player starts paused. Controls apply to reference-timeline positions:

| Key | Action |
| --- | --- |
| Space | Play/pause |
| Left / Right | Seek backward/forward five seconds |
| 0 | Cycle reference, set A, set B (available sources only) |
| 1–6 | Mute bass, drums, guitar, piano, vocals, other |
| Tab | Select the next instrument role |
| s | Toggle solo on the selected role |
| - / = | Lower/raise selected gain by ten percentage points |
| [ | Set loop start at the current position |
| ] | Set loop end and enable the loop |
| l | Clear the loop |
| , / . | Lower/raise playback speed by 0.25, within 0.25–2× |
| ; / ' | Lower/raise pitch one semitone, within ±12 |
| m | Toggle metronome (requires `--bpm`) |
| c | Toggle count-in (requires `--bpm`) |
| Esc / Ctrl-C | Exit |

Mute takes precedence over solo. Multiple solos play their union; absent soloed
roles contribute no audio. Gain is attenuation only, 0–100%. Reference playback
bypasses role controls but remembers them for the stem sets. Selecting a missing
role never silently substitutes another role. Read failures stop playback and
display an error.

For metronome/count-in, pass `--bpm 120 --meter 4/4` with the recording's actual
tempo and meter. This creates a constant-tempo map containing complete measures;
the final partial measure remains uncounted. It does not import authored maps.
Count-in uses the production implementation and stays at the same pitch as speed
and music pitch change.

## Listening checklist

Start with conservative device volume. Record output device and sample rate.

1. With every stem enabled and gain at 100%, cycle reference/A/B at 1×. Check
   onset alignment, relative levels, transients, clicks and gaps.
2. Mute vocals with 5, cycle sources, then select roles with Tab and exercise
   solo and intermediate gain. Reference should ignore instrument controls.
3. Repeat at 0.5× and 0.25× with ,; compare transient smearing and artifacts.
4. Shift pitch in both directions using ;/' at slow and normal speeds.
5. Seek during playback; set [ and ] around a phrase. Switch and mute rapidly
   while looping. Check the phrase position and loop bounds are preserved.
6. Pause, change source/mix, then resume. Changes must not start playback.
7. With BPM/meter supplied, enable metronome/count-in. Change sources during
   count-in and while looping; clicks should keep their phase and fixed pitch.
8. Note interruptions, audible delays and clicks separately from separation
   artifacts. Report whether shared DSP is acceptable for practice.

Automated correctness/performance results, hardware playback and subjective
listening are separate evidence. Software consumer timing does not measure
physical speaker latency. Linux and Windows results must be reported only when
actually exercised; configured CI alone is not proof.

## Reproducible measurements

Run these as separate processes from the repository root:

```sh
CARGO_HOME="$PWD/.cargo-home" cargo run --locked --release --example stem_benchmark -- 15
CARGO_HOME="$PWD/.cargo-home" cargo run --locked --release --example stem_benchmark -- 300
CARGO_HOME="$PWD/.cargo-home" cargo run --locked --release --example stem_benchmark -- 15 --hardware
```

The benchmark generates immutable inputs on disk: a 44.1 kHz reference and two
six-role 48 kHz sets. It begins process RSS measurement after fixture generation,
then measures preparation plus playback. `ps` samples resident memory every 10 ms
on macOS/Linux; it can miss shorter allocation peaks and is not a Windows memory
measurement. Each run reports baseline, observed peak, incremental peak,
preparation time and prepared PCM bytes. External filesystem cache is excluded.

The software consumer runs independently of the control thread and consumes
480 frames every 10 ms at 48 kHz. Changes are detected from actual audio RMS,
requiring three consecutive output windows with the changed signature. Reported
latency includes the command, re-priming and this conservative 30 ms confirmation;
it is not physical speaker latency. Repetitions exercise gain-zero, mute and solo,
regain and A/B switching at 1×/0 semitones, 0.5×/−5 and 0.25×/2× with ±12.
Eight-command bursts are included in worst-case timing. Silent output windows
during source switching are reported separately; sub-window fades/gaps require
listening. Steady 200 ms underrun windows and change-period underruns are separate
from lifetime counters, which also include startup/control transitions.

The hardware mode plays generated stems for eight seconds, including source,
gain, slow/pitch-shifted playback and seeking. It reports the actual device,
callbacks, protection counters and errors. Passing this exercise does not imply
subjective quality approval or validate the full listening checklist.

### Observed on macOS arm64, 2026-10-13

| Generated duration | Baseline RSS | Observed peak RSS | Incremental peak | Preparation | Temporary PCM | Worst normal / other-control latency |
| --- | --- | --- | --- | --- | --- | --- |
| 15 s | 6,733,824 B | 23,920,640 B | 17,186,816 B | 0.343 s | 74,880,000 B | 42.515 / 61.888 ms |
| 300 s | 6,733,824 B | 30,359,552 B | 23,625,728 B | 5.883 s | 1,497,600,000 B | 49.734 / 60.128 ms |

Both runs passed the 256 MiB incremental budget and 250/500 ms latency limits.
The 20× duration increase scales disk use, with both observed memory peaks below
24 MiB incremental. These are sampled observations, not a formal allocator bound.
All measured steady and change-period underrun deltas were zero, and source-switch
silence did not fill a complete 10 ms measurement window. Lifetime underrun
counters were 3 and 6 respectively; do not read the narrower zero counters as
zero lifetime underruns. Both ordinary runs reported zero clipped samples;
pre-clamp peaks were 0.148751 and 0.175253.

Independent fresh-process reruns used `/usr/bin/time -l` to corroborate the
sampled RSS peak with the OS high-water measurement. The 15-second run observed
27,754,496 bytes maximum RSS (21,020,672 bytes sampled incremental peak);
the 300-second run observed 31,309,824 bytes maximum RSS (24,559,616 bytes
sampled incremental peak). Worst normal/other latency was 43.141/61.156 ms and
52.793/60.211 ms respectively. Both remained within the agreed limits, with
zero change-period underruns and clipping. Logs are under
`/tmp/ohmyloop-issue-4-review/parent-benchmark-{15,300}{,-time}.log` for this session.

Additional protection tests feed a prepared six-role transient mix with a
measured 0.70 input peak through real R3, the production worker and Consumer,
with maximum metronome/count-in volume. Across 0.25/0.5/1/2× and −12/0/+12
semitones, the worst pre-clamp peak was 0.770818, with zero clipped samples.
These generated cases establish tested headroom, not a universal overshoot
bound. A deliberately unattenuated test source bypasses preparation and passes
through actual R3: the observed pre-clamp peak was 2.099363 and 63,102 samples
were clamped, while delivered samples remained finite and within ±1. This
verifies the final protection without injecting artificial queued output.

Generated hardware smoke passed on KT_USB_AUDIO at 48 kHz: 1,505 callbacks,
zero underruns/device errors/clipped samples and a 0.120774 pre-clamp peak.
This was a device exercise, not a subjective listening verdict. Linux and Windows
were not exercised in this pass.

The existing two BONES output directories also passed reference/provenance,
hash and geometry validation. Their 296.4-second comparison prepared at 48 kHz
in 19.90 seconds, using 1,411.3 MiB temporary PCM and −7.19 dB common attenuation.
The current local reference is at:

```sh
REFERENCE='/Users/karsten.rohweder/Music/OhMyLoop Library/songs/72a72162-dd7e-4e36-a704-556972372119/assets/6e9a9d2a-a60b-41fa-8240-b554b1c9806d/AMORPHIS_-_Bones__Official_Music_Video_.m4a'
CARGO_HOME="$PWD/.cargo-home" cargo run --locked --release --example stem_player -- \
  "$REFERENCE" artifacts/stem-prototype/bones-demucs-6s \
  artifacts/stem-prototype/bones-roformer-sw
```

That path is machine-specific and should be replaced after moving the library.
Preparation success does not establish perceptual onset alignment or listening
acceptance. ADR 0009 remains awaiting the maintainer's listening approval.
