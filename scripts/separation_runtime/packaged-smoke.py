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
import sys
import threading
import time

ROOT = Path(__file__).resolve().parents[2]
TIMEOUT_SECONDS = 1800


def launch(command: list, env: dict) -> tuple[int, str, int | None]:
    """Run one smoke mode; return exit code, output and peak process RSS in bytes.

    The peak is the largest resident set of any single process in the reaped tree
    (the app or a descendant such as the inference worker), read from this child's
    wait4 rusage. It is not the combined memory of concurrently running processes.
    Unavailable (None) where the platform has no wait4, e.g. Windows.
    """
    process = subprocess.Popen(
        command,
        env=env,
        encoding="utf-8",
        errors="replace",
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
    )
    chunks: list[str] = []
    reader = threading.Thread(target=lambda: chunks.extend(process.stdout), daemon=True)
    reader.start()
    if not hasattr(os, "wait4"):
        try:
            code = process.wait(timeout=TIMEOUT_SECONDS)
        except subprocess.TimeoutExpired:
            process.kill()
            process.wait()
            raise
        reader.join()
        return code, "".join(chunks), None
    deadline = time.monotonic() + TIMEOUT_SECONDS
    while True:
        pid, status, usage = os.wait4(process.pid, os.WNOHANG)
        if pid:
            break
        if time.monotonic() > deadline:
            process.kill()
            os.wait4(process.pid, 0)
            raise subprocess.TimeoutExpired(command, TIMEOUT_SECONDS)
        time.sleep(0.1)
    # Popen did not reap the child itself; record its status so it is not reused.
    process.returncode = os.waitstatus_to_exitcode(status)
    reader.join()
    scale = 1 if platform.system() == "Darwin" else 1024  # Linux reports KiB.
    return process.returncode, "".join(chunks), usage.ru_maxrss * scale


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


# The extracted AppImage starts through linuxdeploy's `#!/usr/bin/env bash`
# AppRun and its hook scripts. An explicit allowlist of basic shell utilities
# lets it start while Python, compilers and FFmpeg stay undiscoverable.
LAUNCHER_TOOLS = (
    "bash", "sh", "env", "dirname", "basename", "readlink", "realpath", "cat",
    "grep", "sed", "cut", "tr", "head", "ls", "mkdir", "uname", "printf", "echo",
)
FORBIDDEN_TOOL_PREFIXES = ("python", "pip", "uv", "cc", "gcc", "g++", "c++", "clang", "ffmpeg", "ffprobe")


def launcher_path(destination: Path, binary: Path) -> str:
    """Return the application's PATH: empty except for the Linux launcher tools."""
    if platform.system() != "Linux":
        return ""
    for tool in LAUNCHER_TOOLS:
        if tool.startswith(FORBIDDEN_TOOL_PREFIXES):
            raise ValueError(f"Launcher allowlist must not expose {tool}")
    tools = destination / "launcher-tools"
    tools.mkdir()
    for tool in LAUNCHER_TOOLS:
        found = shutil.which(tool)
        if found is not None:
            (tools / tool).symlink_to(found)
    # Record exactly what the launcher runs, for diagnosing missing tools.
    for script in [binary, *sorted((binary.parent / "apprun-hooks").glob("*"))]:
        try:
            text = script.read_text(encoding="utf-8")
        except (OSError, UnicodeDecodeError):
            print(f"launcher {script.name}: binary", flush=True)
            continue
        print(f"launcher {script.name}:\n{text}", flush=True)
    return str(tools)


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", required=True, type=Path)
    parser.add_argument(
        "--mode",
        action="append",
        help="Run a selected mode; default executes the complete suite",
    )
    parser.add_argument(
        "--source",
        type=Path,
        help="Audio file for song modes instead of generated 1 s/20 s audio "
        "(e.g. a full song for resource measurement)",
    )
    args = parser.parse_args()
    source = None
    if args.source is not None:
        source = args.source.resolve()
        if not source.is_file():
            parser.error(f"--source is not a file: {source}")
    destination = args.root.resolve()
    if destination.exists():
        parser.error("--root must be a fresh directory; existing data is never removed")
    destination.mkdir(parents=True)
    binary = install(destination)
    profile = destination / "Fresh app data 雪"
    env = dict(os.environ)
    # App image launcher/native GUI can still use OS libraries; no Python,
    # compiler or system FFmpeg is discoverable through PATH by the application.
    env["PATH"] = launcher_path(destination, binary)
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
                original_uv = only(
                    [
                        path
                        for path in destination.rglob(
                            "uv.exe" if platform.system() == "Windows" else "uv"
                        )
                        if path.is_file()
                        and path.parent.name == "tools"
                        and "separation" in path.parts
                    ],
                    "bundled uv",
                )
                original_mode = original_uv.parent.stat().st_mode
                original_uv.parent.chmod(original_mode | 0o200)
                hidden_uv = original_uv.with_name(original_uv.name + ".disabled-for-offline-proof")
                original_uv.rename(hidden_uv)
                original_uv.parent.chmod(original_mode)
                env["HTTPS_PROXY"] = "http://127.0.0.1:9"
                env["HTTP_PROXY"] = "http://127.0.0.1:9"
                env["ALL_PROXY"] = "http://127.0.0.1:9"
                env["NO_PROXY"] = ""
            command = [binary, "--stem-runtime-smoke", mode, profile]
            if source is not None:
                command.append(source)
            start = time.monotonic()
            code, output, peak_rss = launch(command, env)
            (destination / f"{mode}.log").write_text(output, encoding="utf-8")
            results.append(
                {
                    "mode": mode,
                    "exit_code": code,
                    "seconds": round(time.monotonic() - start, 3),
                    "acquisition_disabled": hidden_uv is not None,
                    "source": str(source) if source is not None else None,
                    "peak_single_process_rss_bytes": peak_rss,
                }
            )
            (destination / "results.json").write_text(json.dumps(results, indent=2) + "\n")
            print(output, flush=True)
            if code:
                raise SystemExit(f"Packaged runtime smoke failed in {mode}: {code}")
    finally:
        if hidden_uv is not None and original_uv is not None:
            original_uv.parent.chmod(original_mode | 0o200)
            hidden_uv.rename(original_uv)
            original_uv.parent.chmod(original_mode)
    print(json.dumps(results, indent=2))


if __name__ == "__main__":
    sys.stdout.reconfigure(encoding="utf-8", errors="replace")
    sys.stderr.reconfigure(encoding="utf-8", errors="replace")
    main()
