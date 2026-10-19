"""Protocol-v1, offline child-only Roformer-SW inference. Native owns publication."""
import contextlib
import hashlib
import importlib.metadata
import json
import math
from pathlib import Path
import platform
import socket
import sys
import time

PACKAGE = "roformer-sw-v1"
PROFILE = "roformer-sw-cpu2-v1"
ROLES = {"bass", "drums", "guitar", "piano", "vocals", "other"}
VERSIONS = {"audio-separator": "0.41.1", "torch": "2.8.0", "torchaudio": "2.8.0", "numpy": "2.2.6", "soundfile": "0.13.1", "librosa": "0.11.0"}
if sys.platform != "darwin":
    VERSIONS.update(torch="2.8.0+cpu", torchaudio="2.8.0+cpu")


def runtime():
    actual = {name: importlib.metadata.version(name) for name in VERSIONS}
    if platform.python_version() != "3.12.13" or actual != VERSIONS:
        raise ValueError("Runtime versions differ from the pinned CPU runtime")
    return actual


def sha(path):
    digest = hashlib.sha256()
    with Path(path).open("rb") as stream:
        for block in iter(lambda: stream.read(256 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


def bounded_json(path):
    path = Path(path)
    if path.stat().st_size > 1024 * 1024:
        raise ValueError("Request exceeds protocol limit")
    return json.loads(path.read_text())


def progress(stream, job, stage, completed=None, total=None, unit=None):
    print(json.dumps({"version": 1, "job_id": job, "stage": stage, "completed": completed, "total": total, "unit": unit}, separators=(",", ":")), file=stream, flush=True)


def main():
    if sys.argv[1:] == ["--probe"]:
        packages = runtime()
        import io
        import shutil
        import subprocess
        if not shutil.which("ffmpeg"):
            raise ValueError("FFmpeg is required on PATH by the pinned adapter")
        subprocess.run([shutil.which("ffmpeg"), "-version"], check=True, timeout=10, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        # Real imports/operations catch missing native libraries, CPU and ABI faults.
        def deny_network(*args, **kwargs):
            raise RuntimeError("Runtime checks are offline")
        socket.socket.connect = deny_network
        socket.create_connection = deny_network
        with contextlib.redirect_stdout(sys.stderr):
            import numpy as np
            import soundfile as sf
            import torch
            import torchaudio
            import onnxruntime
            onnxruntime.disable_telemetry_events()
            from audio_separator.separator import Separator
            torch.set_num_threads(2)
            torch.set_num_interop_threads(1)
            sample = torch.arange(512, dtype=torch.float32).reshape(1, -1) / 512
            value = torchaudio.functional.resample(sample, 44100, 48000)
            if not torch.isfinite(value).all() or not torch.isfinite(sample @ sample.T).all():
                raise ValueError("Native CPU operation failed")
            stream = io.BytesIO()
            sf.write(stream, np.zeros((128, 2), dtype=np.float32), 44100, format="WAV", subtype="FLOAT")
            stream.seek(0)
            decoded, rate = sf.read(stream, dtype="float32", always_2d=True)
            if decoded.shape != (128, 2) or rate != 44100:
                raise ValueError("Native audio operation failed")
        print(json.dumps({"python": platform.python_version(), "packages": packages}))
        return
    request = bounded_json(sys.argv[1])
    if request["version"] != 1 or request["package_id"] != PACKAGE or request["profile_id"] != PROFILE:
        raise ValueError("Unsupported protocol or inference profile")
    if request["settings"] != {"device": "cpu", "threads": 2, "seed": 0, "normalization": "none per stem", "roformer_short_input": "right-zero-pad-to-config-chunk; crop-declared-padding"}:
        raise ValueError("Unsupported inference settings")
    job = request["job_id"]
    output = Path(request["output_dir"])
    model_dir = Path(request["model_dir"])
    prep = request["preparation"]
    if set(request["outputs"]) != ROLES or any(v != k + ".wav" for k, v in request["outputs"].items()):
        raise ValueError("Invalid declared outputs")
    protocol = sys.stdout
    progress(protocol, job, "worker.input")
    versions = runtime()
    # All upstream acquisition paths are forbidden during inference.
    def deny_network(*args, **kwargs):
        raise RuntimeError("Inference is offline; explicitly download the package first")
    socket.socket.connect = deny_network
    socket.create_connection = deny_network
    began = time.perf_counter()
    with contextlib.redirect_stdout(sys.stderr):
        import numpy as np
        import soundfile as sf
        import torch
        import onnxruntime
        onnxruntime.disable_telemetry_events()
        from audio_separator.separator import Separator
        from audio_separator.separator.architectures import mdxc_separator
        torch.set_num_threads(2)
        torch.set_num_interop_threads(1)
        torch.manual_seed(0)
        data, rate = sf.read(request["prepared_path"], dtype="float32", always_2d=True)
        if rate != 44100 or data.shape != (prep["prepared_frames"], 2) or not np.isfinite(data).all() or sha(request["prepared_path"]) != prep["prepared_sha256"]:
            raise ValueError("Prepared input identity or geometry mismatch")
        progress(protocol, job, "worker.load")
        separator = Separator(model_file_dir=str(model_dir), output_dir=str(output), sample_rate=44100, use_autocast=False)
        device = torch.device("cpu")
        separator.torch_device = device
        separator.torch_device_cpu = device
        separator.torch_device_mps = None
        separator.download_model_files = lambda filename: ("BS-Roformer-SW.ckpt", "MDXC", "BS RoFormer SW", str(model_dir / "BS-Roformer-SW.ckpt"), str(model_dir / "BS-Roformer-SW.yaml"))
        separator.load_model("BS-Roformer-SW.ckpt")
        config = separator.model_instance.model_data_cfgdict
        chunk = int(getattr(config.model, "stft_hop_length", config.audio.hop_length)) * (int(config.inference.dim_t) - 1)
        if chunk != 409600 or separator.model_instance.overlap != 8:
            raise ValueError("Pinned chunk or overlap profile changed")
        padding = max(0, chunk - len(data))
        padded = np.pad(data, ((0, padding), (0, 0))).T.copy() if padding else data.T.copy()
        corrections = [{"kind": "roformer-short-input-right-zero-pad-and-crop", "input_frames": len(data), "padded_frames": chunk, "right_padding_frames": padding, "crop_start": 0, "crop_end_exclusive": len(data)}] if padding else []
        total = math.ceil(padded.shape[1] / 352800)
        original_tqdm = mdxc_separator.tqdm
        def measured(iterable, *args, **kwargs):
            expected = range(0, padded.shape[1], 352800)
            if not isinstance(iterable, range) or iterable != expected:
                raise ValueError("Pinned inference iteration changed")
            progress(protocol, job, "worker.infer", 0, total, "chunks")
            for completed, index in enumerate(iterable, 1):
                yield index
                progress(protocol, job, "worker.infer", completed, total, "chunks")
        mdxc_separator.tqdm = measured
        progress(protocol, job, "worker.infer", 0, total, "chunks")
        try:
            with torch.inference_mode():
                raw = separator.model_instance.demix(padded)
        finally:
            mdxc_separator.tqdm = original_tqdm
        progress(protocol, job, "worker.write", 0, 6, "tracks")
        if any(value.shape != padded.shape for value in raw.values()):
            raise ValueError("Raw model output disagrees with padded geometry")
        sources = {name.lower(): value.T[:len(data)] for name, value in raw.items()}
        if set(sources) != ROLES or str(next(separator.model_instance.model_run.parameters()).device) != "cpu":
            raise ValueError("Unexpected model outputs or device")
        stems = {}
        for completed, name in enumerate(sorted(ROLES), 1):
            value = sources[name]
            if value.shape != data.shape or not np.isfinite(value).all():
                raise ValueError("Invalid stem geometry or samples")
            target = output / request["outputs"][name]
            sf.write(target, value, 44100, subtype="FLOAT")
            stems[name] = {"file": target.name, "sha256": sha(target), "frames": len(value), "rate": 44100, "channels": 2}
            progress(protocol, job, "worker.write", completed, 6, "tracks")
        resources = {"peak_rss": None, "peak_rss_unit": "unavailable"}
        try:
            import resource
            resources = {"peak_rss": resource.getrusage(resource.RUSAGE_SELF).ru_maxrss, "peak_rss_unit": "bytes" if sys.platform == "darwin" else "KiB"}
        except ImportError:
            pass
        result = {"version": 1, "job_id": job, "package_id": PACKAGE, "profile_id": PROFILE, "runtime_id": request.get("runtime_id"), "preparation": prep, "actual_device": "cpu", "python": platform.python_version(), "packages": versions, "stems": stems, "corrections": corrections, "elapsed_seconds": time.perf_counter()-began, "resources": resources}
        target = output / "result.json"
        temporary = target.with_suffix(".tmp")
        temporary.write_text(json.dumps(result, sort_keys=True))
        temporary.replace(target)
        progress(protocol, job, "worker.finish")

if __name__ == "__main__":
    main()
