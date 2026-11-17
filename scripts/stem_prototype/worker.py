"""Child-only inference. Inputs are prepared float PCM, never source recordings."""

import importlib.metadata
from pathlib import Path
import platform
import socket
import sys
import time
from common import (
    STEMS,
    audio,
    check_models,
    read_json,
    roformer_input,
    sha,
    write_json,
)


def versions():
    packages = (
        "audio-separator",
        "demucs",
        "torch",
        "torchaudio",
        "numpy",
        "soundfile",
    )
    return {name: importlib.metadata.version(name) for name in packages}


def main():
    job = Path(sys.argv[1])
    request = read_json(job / "request.json")
    settings = request["settings"]
    model_dir = Path(request["model_dir"])
    check_models(model_dir, request["model"])

    # Upstream download fallbacks must fail, even when a local artifact is missing.
    def deny_network(*args, **kwargs):
        raise RuntimeError("Inference is offline; use the explicit download command")

    socket.socket.connect = deny_network
    socket.create_connection = deny_network
    import numpy as np
    import soundfile as sf
    import torch

    torch.set_num_threads(settings["threads"])
    torch.set_num_interop_threads(1)
    torch.manual_seed(settings["seed"])
    data = audio(job / "prepared.wav")
    device = torch.device(settings["device"])
    corrections = []
    began = time.perf_counter()
    print(
        f"Loading {request['model']} on explicit {device}; {len(data)} prepared frames",
        flush=True,
    )
    if request["model"] == "demucs-6s":
        from demucs.pretrained import get_model
        from demucs.apply import apply_model

        model = get_model("htdemucs_6s", repo=model_dir).to(device).eval()
        tensor = torch.from_numpy(data.T.copy())
        reference = tensor.mean(0)
        mean, std = reference.mean(), reference.std()
        # Defined silence handling, avoiding division by zero.
        std = std.clamp_min(1e-8)
        print(
            "Inferring Demucs with shifts=0, overlap=0.25; no per-stem clipping",
            flush=True,
        )
        with torch.inference_mode():
            output = (
                apply_model(
                    model,
                    ((tensor - mean) / std)[None],
                    device=device,
                    shifts=0,
                    split=True,
                    overlap=0.25,
                    progress=True,
                    num_workers=0,
                )[0].cpu()
                * std
                + mean
            )
        sources = {name: output[i].numpy().T for i, name in enumerate(model.sources)}
        actual_device = str(next(model.parameters()).device)
    elif request["model"] == "roformer-sw":
        import onnxruntime

        onnxruntime.disable_telemetry_events()
        from audio_separator.separator import Separator

        separator = Separator(
            model_file_dir=str(model_dir),
            output_dir=str(job),
            sample_rate=44100,
            use_autocast=False,
        )
        # This pin auto-detects MPS/CUDA: override before model instantiation.
        separator.torch_device = device
        separator.torch_device_cpu = torch.device("cpu")
        separator.torch_device_mps = device if device.type == "mps" else None
        separator.download_model_files = lambda filename: (
            "BS-Roformer-SW.ckpt",
            "MDXC",
            "BS RoFormer SW",
            str(model_dir / "BS-Roformer-SW.ckpt"),
            str(model_dir / "BS-Roformer-SW.yaml"),
        )
        separator.load_model("BS-Roformer-SW.ckpt")
        print(
            "Inferring raw MDXC.demix; bypass input/output normalization and WAV writer",
            flush=True,
        )
        config = separator.model_instance.model_data_cfgdict
        hop = getattr(config.model, "stft_hop_length", config.audio.hop_length)
        chunk = int(hop) * (int(config.inference.dim_t) - 1)
        padded, corrections = roformer_input(data, chunk)
        if corrections:
            print(
                f"Explicit short-input right padding: {len(data)} -> {padded.shape[-1]} frames",
                flush=True,
            )
        with torch.inference_mode():
            raw = separator.model_instance.demix(padded)
        if any(value.shape != padded.shape for value in raw.values()):
            raise ValueError("RoFormer output differs from declared padded geometry")
        sources = {name.lower(): value.T[: len(data)] for name, value in raw.items()}
        actual_device = str(
            next(separator.model_instance.model_run.parameters()).device
        )
    else:
        raise ValueError("Unknown real model")
    if set(sources) != STEMS:
        raise ValueError(f"Unexpected stem names: {set(sources)}")
    outputs = {}
    for name, value in sources.items():
        if value.shape != data.shape or not np.isfinite(value).all():
            raise ValueError(
                f"Invalid {name} geometry/non-finite output: {value.shape}; no silent corrections"
            )
        target = job / (name + ".wav")
        sf.write(target, value, 44100, subtype="FLOAT")
        outputs[name] = {
            "file": target.name,
            "sha256": sha(target),
            "frames": len(value),
            "channels": 2,
            "rate": 44100,
            "peak": float(np.abs(value).max()),
        }
    resource_info = {"peak_rss": None, "peak_rss_unit": "unavailable"}
    try:
        import resource

        resource_info = {
            "peak_rss": resource.getrusage(resource.RUSAGE_SELF).ru_maxrss,
            "peak_rss_unit": "bytes" if sys.platform == "darwin" else "KiB",
        }
    except ImportError:
        pass
    write_json(
        job / "worker-result.json",
        {
            "stems": outputs,
            "elapsed_seconds": time.perf_counter() - began,
            "actual_device": actual_device,
            "packages": versions(),
            "python": platform.python_version(),
            "resources": resource_info,
            "corrections": corrections,
            "alignment_evidence": "geometry only; perceptual/sample onset alignment unverified",
        },
    )


if __name__ == "__main__":
    main()
