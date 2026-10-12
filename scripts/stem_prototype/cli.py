#!/usr/bin/env python3
"""Isolated offline stem experiment. Help/models require only the stdlib."""

import argparse
import importlib.metadata
import math
import os
from pathlib import Path
import platform
import shutil
import signal
import subprocess
import sys
import tempfile
import time
import urllib.request
from common import (
    MODELS,
    ROOT,
    STEMS,
    audio,
    check_models,
    identity,
    mix_arrays,
    read_json,
    sha,
    validate_manifest,
    write_json,
)


def run_child(command, env=None):
    """Stream inherited progress; kill/reap on Ctrl-C, SIGTERM or parent exception."""
    child = subprocess.Popen(command, env=env, start_new_session=(os.name != "nt"))
    previous = signal.getsignal(signal.SIGTERM)

    def cancelled(signum, frame):
        raise KeyboardInterrupt

    signal.signal(signal.SIGTERM, cancelled)
    try:
        code = child.wait()
        if code:
            raise RuntimeError(f"Child failed (exit {code})")
    finally:
        signal.signal(signal.SIGTERM, previous)
        if child.poll() is None:
            if os.name != "nt":
                os.killpg(child.pid, signal.SIGTERM)
            else:
                child.terminate()
            try:
                child.wait(timeout=3)
            except subprocess.TimeoutExpired:
                if os.name != "nt":
                    os.killpg(child.pid, signal.SIGKILL)
                else:
                    child.kill()
                child.wait()


def download(args):
    directory = args.models_dir / args.model
    directory.mkdir(parents=True, exist_ok=True)
    lock_path = directory / "model-lock.json"
    if lock_path.exists():
        check_models(directory, args.model)
        print(f"Verified existing model: {directory}")
        return
    # Stage acquisition atomically so failed config/downloads are retryable.
    staging = Path(tempfile.mkdtemp(prefix=".download-", dir=directory.parent))
    try:
        files = {}
        for name, url in MODELS[args.model]["files"].items():
            if (directory / name).exists():
                raise ValueError(
                    f"Unlocked existing artifact: {directory / name}; inspect/remove before retry"
                )
            target = staging / name
            candidates = [url]
            mirror = MODELS[args.model].get("mirrors", {}).get(name)
            if mirror:
                candidates.append(mirror)
            for candidate in candidates:
                print(f"Downloading {candidate}", flush=True)
                try:
                    with (
                        urllib.request.urlopen(candidate, timeout=60) as source,
                        target.open("wb") as destination,
                    ):
                        shutil.copyfileobj(source, destination, 1024 * 1024)
                    url = candidate
                    break
                except OSError:
                    target.unlink(missing_ok=True)
                    if candidate == candidates[-1]:
                        raise
            digest = sha(target)
            expected = (
                MODELS[args.model].get("expected_checkpoint_sha256")
                if name.endswith(".ckpt")
                else None
            )
            prefix = "34c22ccb" if name.endswith(".th") else None
            if (expected and digest != expected) or (
                prefix and not digest.startswith(prefix)
            ):
                raise ValueError(f"Publisher checkpoint checksum mismatch: {name}")
            files[name] = {"url": url, "sha256": digest, "bytes": target.stat().st_size}
        write_json(
            staging / "model-lock.json",
            {
                "version": 1,
                "model": args.model,
                "files": files,
                "acquired_at_utc": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()),
                "provenance": "Explicit upstream/mirror acquisition; SHA256 locked locally. Release config URLs may change.",
            },
        )
        # No checkpoint is visible as acquired until every artifact validates.
        directory.rmdir()
        staging.rename(directory)
    finally:
        if staging.exists():
            shutil.rmtree(staging)
    print(f"Model ready: {lock_path}")


def runtime_identity():
    for name in (
        "audio-separator",
        "demucs",
        "torch",
        "torchaudio",
        "numpy",
        "soundfile",
    ):
        importlib.metadata.version(name)
    return {
        "python": platform.python_version(),
        "platform": platform.platform(),
        "packages": {
            dist.metadata["Name"].lower(): dist.version
            for dist in importlib.metadata.distributions()
        },
        "implementation": {
            name: sha(Path(__file__).parent / name)
            for name in ("cli.py", "common.py", "worker.py")
        },
        "rust_preparer_sha256": sha(ROOT / "examples/stem_prepare.rs"),
        "cargo_lock_sha256": sha(ROOT / "Cargo.lock"),
        "audio_source_sha256": sha(ROOT / "src/audio.rs"),
        "requirements_lock_sha256": sha(Path(__file__).with_name("requirements.lock")),
    }


def separate(args):
    if (
        not math.isfinite(args.start)
        or args.start < 0
        or (not args.whole and (not math.isfinite(args.duration) or args.duration <= 0))
    ):
        raise ValueError("Start/duration must be finite and nonnegative/positive")
    model_dir = (args.models_dir / args.model).resolve()
    model_lock = check_models(model_dir, args.model)
    runtime = runtime_identity()  # Fail before preparation if environment is missing.
    args.jobs_dir.mkdir(parents=True, exist_ok=True)
    working = Path(tempfile.mkdtemp(prefix=".incomplete-", dir=args.jobs_dir))
    began = time.perf_counter()
    try:
        env = os.environ.copy()
        env.update(
            {
                "CARGO_HOME": str(ROOT / ".cargo-home"),
                "TORCH_HOME": str(ROOT / ".tools/stems/torch"),
                "HF_HOME": str(ROOT / ".tools/stems/hf"),
                "XDG_CACHE_HOME": str(ROOT / ".tools/stems/cache"),
                "NUMBA_CACHE_DIR": str(ROOT / ".tools/stems/numba"),
                "OMP_NUM_THREADS": str(args.threads),
                "MKL_NUM_THREADS": str(args.threads),
                "OPENBLAS_NUM_THREADS": str(args.threads),
                "CUDA_VISIBLE_DEVICES": ""
                if args.device == "cpu"
                else env.get("CUDA_VISIBLE_DEVICES", "0"),
                "TORCH_FORCE_NO_WEIGHTS_ONLY_LOAD": "1",
            }
        )
        prepare = working / "preparation"
        run_child(
            [
                "cargo",
                "run",
                "--locked",
                "--release",
                "--example",
                "stem_prepare",
                "--",
                str(args.input.resolve()),
                str(prepare.resolve()),
                str(args.start),
                "whole" if args.whole else str(args.duration),
            ],
            env,
        )
        prep = read_json(prepare / "preparation.json")
        shutil.move(str(prepare / "prepared.wav"), working / "prepared.wav")
        shutil.move(str(prepare / "preparation.json"), working / "preparation.json")
        prepare.rmdir()
        settings = {
            "threads": args.threads,
            "device": args.device,
            "seed": 0,
            "demucs_shifts": 0,
            "demucs_overlap": 0.25,
            "roformer_overlap": 8,
            "roformer_segment": "config default",
            "roformer_short_input": "right-zero-pad-to-config-chunk; crop-declared-padding",
            "precision": "float32; autocast off",
            "normalization": "none per stem",
        }
        key_data = {
            "preparation": prep,
            "model_lock": model_lock,
            "settings": settings,
            "runtime": runtime,
        }
        key = identity(key_data)
        complete = args.jobs_dir / key
        if complete.exists():
            validate_manifest(complete / "manifest.json", key_data)
            print(f"Verified cache hit: {complete / 'manifest.json'}")
            return
        write_json(
            working / "request.json",
            {"model": args.model, "model_dir": str(model_dir), "settings": settings},
        )
        run_child(
            [
                sys.executable,
                str(Path(__file__).with_name("worker.py")),
                str(working.resolve()),
            ],
            env,
        )
        result = read_json(working / "worker-result.json")
        result.update(
            {
                "version": 1,
                "status": "complete",
                "job_key": key,
                "identity": key_data,
                "total_elapsed_seconds": time.perf_counter() - began,
                "source_path_informational": str(args.input.resolve()),
            }
        )
        # Validate staging before the completed manifest name becomes visible.
        staged = working / "manifest.pending.json"
        write_json(staged, result)
        validate_manifest(staged, key_data)
        staged.rename(working / "manifest.json")
        working.rename(complete)
        print(f"Completed: {complete / 'manifest.json'}")
        for name in sorted(STEMS):
            print(f"Open in OhMyLoop: {complete / (name + '.wav')}")
    finally:
        if working.exists():
            shutil.rmtree(working)


def mix(args):
    import soundfile as sf

    manifest = validate_manifest(args.manifest)
    directory = args.manifest.parent
    frames = manifest["identity"]["preparation"]["prepared_frames"]
    sources = {
        name: audio(directory / info["file"], frames)
        for name, info in manifest["stems"].items()
    }
    gains = {}
    for setting in args.gain:
        name, value = setting.split("=", 1)
        gains[name] = float(value)
    output, common_gain, peak = mix_arrays(
        sources, audio(directory / "prepared.wav", frames), args.preset, gains
    )
    target = args.output or directory / (args.preset + ".wav")
    if target.exists() or target.with_suffix(".mix.json").exists():
        raise ValueError(f"Mix output or metadata already exists: {target}")
    target.parent.mkdir(parents=True, exist_ok=True)
    sf.write(target, output, 44100, subtype="FLOAT")
    write_json(
        target.with_suffix(".mix.json"),
        {
            "version": 1,
            "result_key": manifest["job_key"],
            "preset": args.preset,
            "gains": gains,
            "common_gain": common_gain,
            "unscaled_peak": peak,
            "policy": "one common attenuation only when peak exceeds0.98; no stem normalization",
            "sha256": sha(target),
        },
    )
    print(f"Open in OhMyLoop: {target}; common gain={common_gain:.6f}")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--models-dir", type=Path, default=ROOT / ".tools/stems/models")
    parser.add_argument(
        "--jobs-dir", type=Path, default=ROOT / "artifacts/stem-prototype"
    )
    commands = parser.add_subparsers(dest="command", required=True)
    commands.add_parser("models")
    fetch = commands.add_parser("download")
    fetch.add_argument("--model", choices=MODELS, required=True)
    sep = commands.add_parser("separate")
    sep.add_argument("input", type=Path)
    sep.add_argument("--model", choices=MODELS, required=True)
    sep.add_argument("--start", type=float, default=0)
    span = sep.add_mutually_exclusive_group()
    span.add_argument("--duration", type=float, default=20)
    span.add_argument("--whole", action="store_true")
    sep.add_argument("--threads", type=int, choices=range(1, 9), default=2)
    sep.add_argument("--device", choices=("cpu", "mps", "cuda"), default="cpu")
    mixer = commands.add_parser("mix")
    mixer.add_argument("manifest", type=Path)
    mixer.add_argument(
        "--preset",
        choices=("guitar-only", "no-guitar", "sum", "original-minus-guitar"),
        default="no-guitar",
    )
    mixer.add_argument(
        "--gain", action="append", default=[], metavar="STEM=LINEAR_GAIN"
    )
    mixer.add_argument("--output", type=Path)
    args = parser.parse_args()
    if args.command == "models":
        for name, info in MODELS.items():
            try:
                check_models(args.models_dir / name, name)
                status = "verified local artifacts"
            except (OSError, ValueError, KeyError):
                status = "not acquired or invalid"
            print(f"{name}: {info['description']} [{status}]")
    elif args.command == "download":
        download(args)
    elif args.command == "separate":
        separate(args)
    else:
        mix(args)


if __name__ == "__main__":
    try:
        main()
    except KeyboardInterrupt:
        print(
            "Cancelled; child reaped and incomplete result discarded", file=sys.stderr
        )
        sys.exit(130)
    except (
        ValueError,
        OSError,
        RuntimeError,
        KeyError,
        importlib.metadata.PackageNotFoundError,
    ) as error:
        print(f"Error: {error}", file=sys.stderr)
        sys.exit(1)
