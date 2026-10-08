#!/usr/bin/env python3
"""Optional real encode/decode smoke test using original generated material."""
import json
import shutil
import subprocess
import sys
from pathlib import Path

root = Path(__file__).resolve().parents[1]
binary = root / "target" / "release" / ("ohmyloop.exe" if sys.platform == "win32" else "ohmyloop")
directory = root / "artifacts" / "codecs"
if not binary.exists():
    raise SystemExit("Build the release binary first: cargo build --release")
if not shutil.which("ffmpeg") or not shutil.which("ffprobe"):
    raise SystemExit("This optional check requires ffmpeg and ffprobe on PATH")
directory.mkdir(parents=True, exist_ok=True)

def run(args):
    try:
        return subprocess.run([str(x) for x in args], check=True, text=True, capture_output=True).stdout
    except subprocess.CalledProcessError as error:
        raise SystemExit(error.stderr.strip() or f"Command failed: {args}") from error

original = directory / "original.wav"
run([binary, "demo", original])
results = []
encoders = run(["ffmpeg", "-hide_banner", "-encoders"])
vorbis = "libvorbis" if " libvorbis " in encoders else "vorbis"
for name, codec, extension in [
    ("PCM", "pcm_s16le", "wav"),
    ("FLAC", "flac", "flac"),
    ("MP3", "libmp3lame", "mp3"),
    ("AAC-LC", "aac", "m4a"),
    ("ALAC", "alac", "m4a"),
    ("Vorbis", vorbis, "ogg"),
]:
    encoded = directory / f"{name}.{extension}"
    decoded = directory / f"{name}-rendered.wav"
    # Native FFmpeg Vorbis is an experimental encoder; it is only a test
    # fixture generator, not part of the app's decoding or playback path.
    extra = ["-strict", "-2"] if codec == "vorbis" else []
    run(["ffmpeg", "-v", "error", "-y", "-i", original, "-ar", "44100", "-c:a", codec, *extra, encoded])
    run([binary, "render", encoded, decoded, "--speed", "0.75", "--pitch", "3", "--seconds", "2"])
    data = json.loads(run(["ffprobe", "-v", "error", "-show_streams", "-of", "json", decoded]))["streams"][0]
    passed = int(data["sample_rate"]) == 44100 and data["channels"] == 2 and abs(float(data["duration"])-2.0) < 0.002
    results.append({"codec": name, "passed": passed, "output_seconds": float(data["duration"]), "rate": int(data["sample_rate"]), "channels": data["channels"]})
    print(f"{name}: {'PASS' if passed else 'FAIL'}")
(directory / "measurements.json").write_text(json.dumps(results, indent=2)+"\n")
if not all(row["passed"] for row in results):
    raise SystemExit("Codec checks failed")
print(f"Original fixtures and results: {directory}")
