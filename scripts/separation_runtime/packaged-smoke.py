#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-or-later
"""Install/relocate a built artifact and exercise its actual managed-runtime API.

This script is maintainer/CI infrastructure. The installed application never
needs Python to launch setup. All smoke input and data are private to --root.
"""
from __future__ import annotations

import argparse
import json
import os
from pathlib import Path
import platform
import shutil
import subprocess
import time

ROOT = Path(__file__).resolve().parents[2]


def only(paths: list[Path], description: str) -> Path:
    if len(paths) != 1:
        raise ValueError(f"Expected one {description}, got {paths}")
    return paths[0]


def install(destination: Path) -> Path:
    bundles = ROOT / "target" / "release" / "bundle"
    system = platform.system()
    if system == "Darwin":
        source = only(list((bundles / "macos").glob("*.app")), "macOS application")
        app = destination / "OhMyLoop relocated 雪.app"
        shutil.copytree(source, app, symlinks=True)
        binary = app / "Contents" / "MacOS" / "ohmyloop-desktop"
    elif system == "Windows":
        installer = only(list((bundles / "nsis").glob("*.exe")), "NSIS installer")
        app = destination / "OhMyLoop installed 雪"
        subprocess.run([installer, "/S", f"/D={app}"], check=True, timeout=120)
        binary = app / "ohmyloop-desktop.exe"
        if not binary.is_file():
            binary = only(list(app.glob("*.exe")), "installed application executable")
    elif system == "Linux":
        artifact = only(list((bundles / "appimage").glob("*.AppImage")), "AppImage")
        subprocess.run(
            [artifact, "--appimage-extract"],
            cwd=destination,
            check=True,
            stdout=subprocess.DEVNULL,
            timeout=120,
        )
        app = destination / "OhMyLoop extracted 雪"
        (destination / "squashfs-root").rename(app)
        binary = app / "AppRun"
    else:
        raise ValueError(f"Unsupported runtime smoke host: {system}")
    if not binary.is_file():
        raise ValueError(f"Installed executable missing: {binary}")
    # Resource files must be read-only; executable bits remain available. Do not
    # chmod through any AppImage/application symlink into another directory.
    if system != "Windows":
        for path in app.rglob("*"):
            if path.is_symlink():
                continue
            mode = path.stat().st_mode
            path.chmod(0o555 if path.is_dir() or mode & 0o111 else 0o444)
    return binary


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", required=True, type=Path)
    parser.add_argument(
        "--mode",
        action="append",
        help="Run a selected mode; default executes the complete suite",
    )
    args = parser.parse_args()
    destination = args.root.resolve()
    if destination.exists():
        parser.error("--root must be a fresh directory; existing data is never removed")
    destination.mkdir(parents=True)
    binary = install(destination)
    profile = destination / "Fresh app data 雪"
    env = dict(os.environ)
    # App image launcher/native GUI can still use OS libraries; no Python,
    # compiler or system FFmpeg is discoverable through PATH by the application.
    env["PATH"] = ""
    for key in list(env):
        if key.startswith(("PYTHON", "UV_", "PIP_", "CONDA", "VIRTUAL_ENV")):
            del env[key]
    modes = args.mode or [
        "cancel-setup",
        "shutdown-setup",
        "setup",
        "check",
        "run-short",
        "run-normal",
        "cancel-run",
        "shutdown-run",
        "portable",
        "remove-tools",
        "remove-model",
        "verify-completed",
    ]
    results = []
    hidden_uv = None
    original_uv = None
    original_mode = None
    try:
        for mode in modes:
            if mode == "check" and hidden_uv is None:
                # Only mutate the isolated installed copy. A missing bundled uv
                # makes any accidental dependency synchronization fail outright.
                original_uv = only(list(destination.rglob("uv.exe" if platform.system() == "Windows" else "uv")), "bundled uv")
                original_mode = original_uv.parent.stat().st_mode
                original_uv.parent.chmod(original_mode | 0o200)
                hidden_uv = original_uv.with_name(original_uv.name + ".disabled-for-offline-proof")
                original_uv.rename(hidden_uv)
                original_uv.parent.chmod(original_mode)
                env["HTTPS_PROXY"] = "http://127.0.0.1:9"
                env["HTTP_PROXY"] = "http://127.0.0.1:9"
                env["ALL_PROXY"] = "http://127.0.0.1:9"
                env["NO_PROXY"] = ""
            start = time.monotonic()
            result = subprocess.run(
                [binary, "--stem-runtime-smoke", mode, profile],
                env=env,
                text=True,
                stdout=subprocess.PIPE,
                stderr=subprocess.STDOUT,
                timeout=1800,
            )
            (destination / f"{mode}.log").write_text(result.stdout, encoding="utf-8")
            results.append({"mode": mode, "exit_code": result.returncode, "seconds": round(time.monotonic() - start, 3), "acquisition_disabled": hidden_uv is not None})
            (destination / "results.json").write_text(json.dumps(results, indent=2) + "\n")
            print(result.stdout, flush=True)
            if result.returncode:
                raise SystemExit(f"Packaged runtime smoke failed in {mode}: {result.returncode}")
    finally:
        if hidden_uv is not None and original_uv is not None:
            original_uv.parent.chmod(original_mode | 0o200)
            hidden_uv.rename(original_uv)
            original_uv.parent.chmod(original_mode)
    print(json.dumps(results, indent=2))


if __name__ == "__main__":
    main()
