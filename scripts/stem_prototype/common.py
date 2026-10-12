"""Manifest boundary and mixing; inference libraries are deliberately lazy."""

import hashlib
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
MODELS = {
    "demucs-6s": {
        "description": "HTDemucs six stems (direct demucs4.0.1); CPU baseline",
        "files": {
            "5c90dfd2-34c22ccb.th": "https://dl.fbaipublicfiles.com/demucs/hybrid_transformer/5c90dfd2-34c22ccb.th",
            "htdemucs_6s.yaml": "https://raw.githubusercontent.com/facebookresearch/demucs/e976d93ecc3865e5757426930257e200846a520a/demucs/remote/htdemucs_6s.yaml",
        },
    },
    "roformer-sw": {
        "description": "BS-RoFormer-SW six stems via audio-separator0.41.1 raw MDXC.demix",
        "files": {
            "BS-Roformer-SW.ckpt": "https://github.com/TRvlvr/model_repo/releases/download/all_public_uvr_models/BS-Roformer-SW.ckpt",
            "BS-Roformer-SW.yaml": "https://github.com/nomadkaraoke/python-audio-separator/releases/download/model-configs/BS-Roformer-SW.yaml",
        },
        "mirrors": {
            "BS-Roformer-SW.ckpt": "https://huggingface.co/lainlives/audio-separator-models/resolve/3b39120409f3c2d50e9cc4c391f4169131e9d643/BS-Roformer-SW.ckpt",
            "BS-Roformer-SW.yaml": "https://huggingface.co/lainlives/audio-separator-models/resolve/3b39120409f3c2d50e9cc4c391f4169131e9d643/BS-Roformer-SW.yaml",
        },
        "expected_checkpoint_sha256": "24e7d35ee9c64415673d3fd33e06a67cac2c103c5df6267ba1576459c775916e",
    },
}
STEMS = {"bass", "drums", "other", "vocals", "guitar", "piano"}


def sha(path):
    digest = hashlib.sha256()
    with Path(path).open("rb") as stream:
        for block in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


def read_json(path):
    return json.loads(Path(path).read_text())


def write_json(path, value):
    path = Path(path)
    temp = path.with_suffix(path.suffix + ".tmp")
    temp.write_text(json.dumps(value, indent=2, sort_keys=True) + "\n")
    temp.replace(path)


def identity(value):
    return hashlib.sha256(
        json.dumps(value, sort_keys=True, separators=(",", ":")).encode()
    ).hexdigest()


def safe_file(directory, name):
    if not isinstance(name, str) or Path(name).name != name or name in {"", ".", ".."}:
        raise ValueError("Unsafe manifest filename")
    path = directory / name
    if path.is_symlink() or not path.is_file():
        raise ValueError(f"Missing or unsafe file: {name}")
    return path


def check_models(model_dir, model):
    lock = read_json(model_dir / "model-lock.json")
    if lock.get("version") != 1 or lock.get("model") != model:
        raise ValueError("Invalid model lock; explicitly download first")
    if set(lock["files"]) != set(MODELS[model]["files"]):
        raise ValueError("Incomplete model lock")
    for name, info in lock["files"].items():
        digest = sha(safe_file(model_dir, name))
        expected = (
            MODELS[model].get("expected_checkpoint_sha256")
            if name.endswith(".ckpt")
            else None
        )
        if (expected and digest != expected) or (
            name.endswith(".th") and not digest.startswith("34c22ccb")
        ):
            raise ValueError(f"Publisher checkpoint checksum mismatch: {name}")
        if digest != info["sha256"]:
            raise ValueError(f"Model hash mismatch: {name}")
    return lock


def audio(path, frames=None):
    import numpy as np
    import soundfile as sf

    data, rate = sf.read(path, dtype="float32", always_2d=True)
    if (
        rate != 44100
        or data.shape[1] != 2
        or (frames is not None and len(data) != frames)
    ):
        raise ValueError(f"Invalid audio geometry: {path}, {data.shape}, {rate}")
    if not len(data) or not np.isfinite(data).all():
        raise ValueError(f"Empty or non-finite audio: {path}")
    return data


def validate_manifest(path, expected=None):
    path = Path(path)
    result = read_json(path)
    if result.get("version") != 1 or result.get("status") != "complete":
        raise ValueError("Unsupported or incomplete result")
    if result.get("job_key") != identity(result["identity"]):
        raise ValueError("Invalid result identity")
    if expected is not None and result["identity"] != expected:
        raise ValueError("Cache identity mismatch")
    if set(result["stems"]) != STEMS:
        raise ValueError("Unexpected stem set")
    prep = result["identity"]["preparation"]
    if (
        prep.get("version") != 1
        or prep.get("prepared_rate") != 44100
        or prep.get("channels") != 2
    ):
        raise ValueError("Invalid preparation")
    frames = prep["prepared_frames"]
    if not isinstance(frames, int) or frames <= 0:
        raise ValueError("Invalid frame count")
    source_rate, source_frames = prep.get("source_rate"), prep.get("source_frames")
    span = prep.get("source_span", {})
    first, end = span.get("start"), span.get("end_exclusive")
    if (
        any(type(v) is not int for v in (source_rate, source_frames, first, end))
        or source_rate <= 0
        or not 0 <= first < end <= source_frames
        or frames != (end - first) * 44100 // source_rate
    ):
        raise ValueError("Invalid source span or resampling geometry")
    prepared = safe_file(path.parent, "prepared.wav")
    if sha(prepared) != prep["prepared_sha256"]:
        raise ValueError("Prepared PCM hash mismatch")
    audio(prepared, frames)
    for stem, info in result["stems"].items():
        if (info.get("frames"), info.get("channels"), info.get("rate")) != (
            frames,
            2,
            44100,
        ):
            raise ValueError(f"Declared stem geometry mismatch: {stem}")
        output = safe_file(path.parent, info["file"])
        if sha(output) != info["sha256"]:
            raise ValueError(f"Output hash mismatch: {stem}")
        audio(output, frames)
    return result


def mix_arrays(stems, original, preset, gains, headroom=0.98):
    import numpy as np

    if preset == "original-minus-guitar":
        if gains:
            raise ValueError("Gains cannot be combined with subtraction residual")
        mixed = original.astype("float64") - stems["guitar"]
    else:
        defaults = {
            name: (
                float(name == "guitar")
                if preset == "guitar-only"
                else float(name != "guitar")
                if preset == "no-guitar"
                else 1.0
            )
            for name in STEMS
        }
        if set(gains) - STEMS or any(
            not np.isfinite(v) or abs(v) > 16 for v in gains.values()
        ):
            raise ValueError("Unknown stem or gain outside finite [-16,16]")
        defaults.update(gains)
        mixed = sum(
            stems[name].astype("float64") * defaults[name] for name in sorted(STEMS)
        )
    if not np.isfinite(mixed).all():
        raise ValueError("Non-finite mix")
    peak = float(np.max(np.abs(mixed)))
    gain = min(1.0, headroom / peak) if peak else 1.0
    return (mixed * gain).astype("float32"), gain, peak


def roformer_input(data, chunk_frames):
    """Explicit right padding for the pinned adapter's short-passage limitation."""
    import numpy as np

    if not isinstance(chunk_frames, int) or chunk_frames <= 0:
        raise ValueError("Invalid RoFormer chunk length")
    added = max(0, chunk_frames - len(data))
    if not added:
        return data.T.copy(), []
    padded = np.pad(data, ((0, added), (0, 0)))
    return padded.T.copy(), [
        {
            "kind": "roformer-short-input-right-zero-pad-and-crop",
            "input_frames": len(data),
            "padded_frames": len(padded),
            "right_padding_frames": added,
            "crop_start": 0,
            "crop_end_exclusive": len(data),
        }
    ]
