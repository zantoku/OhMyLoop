#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-or-later
"""Maintainer-side tool assembly. Never executed by the installed application.

Downloads pinned bootstrap tools and builds only source-only wheels. Runtime
Python and the inference dependency environment are assembled by the application.
Requires Python 3.12, a native C compiler (diffq on Unix), and HTTPS access.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import os
from pathlib import Path
import platform
import shutil
import subprocess
import tarfile
import urllib.parse
import urllib.request
import zipfile

ROOT = Path(__file__).resolve().parents[2]
TARGETS = {
    ("Darwin", "arm64"): "aarch64-apple-darwin",
    ("Linux", "x86_64"): "x86_64-unknown-linux-gnu",
    ("Windows", "AMD64"): "x86_64-pc-windows-msvc",
}


def digest(path: Path) -> str:
    with path.open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


def download(artifact: dict, cache: Path) -> Path:
    url = artifact["url"]
    if urllib.parse.urlparse(url).scheme != "https":
        raise ValueError("Artifacts must use HTTPS")
    path = cache / artifact["filename"]
    if path.is_file() and digest(path) == artifact["sha256"]:
        return path
    temporary = path.with_suffix(path.suffix + ".partial")
    print(f"Acquiring {artifact['filename']}", flush=True)
    try:
        with urllib.request.urlopen(url, timeout=60) as source, temporary.open(
            "wb"
        ) as out:
            shutil.copyfileobj(source, out)
        if digest(temporary) != artifact["sha256"]:
            raise ValueError(f"Checksum mismatch: {artifact['filename']}")
        expected = artifact.get("bytes", 0)
        if expected and temporary.stat().st_size != expected:
            raise ValueError(f"Length mismatch: {artifact['filename']}")
        temporary.replace(path)
    finally:
        temporary.unlink(missing_ok=True)
    return path


def copy_member(archive: Path, suffix: str, output: Path) -> None:
    if archive.suffix in {".zip", ".whl"}:
        with zipfile.ZipFile(archive) as source:
            names = [
                n
                for n in source.namelist()
                if n == suffix.lstrip("/") or n.endswith(suffix)
            ]
            if len(names) != 1:
                raise ValueError(f"Expected one {suffix} in {archive}")
            output.write_bytes(source.read(names[0]))
    else:
        with tarfile.open(archive) as source:
            members = [m for m in source.getmembers() if m.name.endswith(suffix)]
            if len(members) != 1 or not members[0].isfile():
                raise ValueError(f"Expected one file {suffix} in {archive}")
            with source.extractfile(members[0]) as stream:
                output.write_bytes(stream.read())
    output.chmod(0o755)


def run(command: list[str], **kwargs) -> None:
    print("Running " + " ".join(map(str, command)), flush=True)
    subprocess.run(list(map(str, command)), check=True, **kwargs)


def prepare(target: str, build_python: str) -> None:
    resource = ROOT / "resources" / "separation" / "runtime" / target
    artifacts = json.loads((resource / "artifacts.json").read_text())
    definition = json.loads((resource / "definition.json").read_text())
    work = ROOT / ".tools" / "runtime-packaging" / target
    cache = work / "downloads"
    tools, wheels, notices = (resource / p for p in ("tools", "wheels", "notices"))
    for directory in (cache, tools, wheels, notices):
        directory.mkdir(parents=True, exist_ok=True)
    for name in (
        "uv-MIT.txt",
        "uv-Apache-2.0.txt",
        "FFmpeg-GPL-2.0.txt",
        "FFmpeg-GPL-3.0.txt",
    ):
        shutil.copyfile(ROOT / "licenses" / name, notices / name)
    windows = target.endswith("windows-msvc")
    uv = tools / ("uv.exe" if windows else "uv")
    copy_member(download(artifacts["uv"], cache), "/uv.exe" if windows else "/uv", uv)
    ffmpeg_archive = download(artifacts["ffmpeg"], cache)
    with zipfile.ZipFile(ffmpeg_archive) as archive:
        binaries = [n for n in archive.namelist() if "/binaries/ffmpeg-" in n]
        if len(binaries) != 1:
            raise ValueError("Expected exactly one FFmpeg binary")
        ffmpeg = tools / ("ffmpeg.exe" if windows else "ffmpeg")
        ffmpeg.write_bytes(archive.read(binaries[0]))
        ffmpeg.chmod(0o755)
        for name in archive.namelist():
            if "license" in name.lower() or name.endswith("/README.md"):
                (notices / ("imageio-" + name.replace("/", "-"))).write_bytes(
                    archive.read(name)
                )
    # Preserve upstream uv licensing separately from the application license.
    uv_archive = cache / artifacts["uv"]["filename"]
    if windows:
        with zipfile.ZipFile(uv_archive) as archive:
            for name in archive.namelist():
                if "license" in name.lower():
                    (notices / ("uv-" + name.replace("/", "-"))).write_bytes(
                        archive.read(name)
                    )
    else:
        with tarfile.open(uv_archive) as archive:
            for member in archive.getmembers():
                if member.isfile() and "license" in member.name.lower():
                    (notices / ("uv-" + member.name.replace("/", "-"))).write_bytes(
                        archive.extractfile(member).read()
                    )
    ff_version = subprocess.check_output([ffmpeg, "-version"], text=True).splitlines()[
        0
    ]
    if definition["ffmpeg_version"] not in ff_version:
        raise ValueError(f"Unexpected bundled FFmpeg: {ff_version}")
    buildenv = work / "buildenv"
    run(
        [
            uv,
            "--no-config",
            "venv",
            "--allow-existing",
            "--python",
            build_python,
            buildenv,
        ]
    )
    python = buildenv / ("Scripts/python.exe" if windows else "bin/python")
    # Build requirements themselves are artifact-locked, and installed without
    # resolution. These tools are maintainer-side only, never shipped as setup.
    build_lock = ROOT / "scripts" / "separation_runtime" / "build-requirements.lock"
    run(
        [
            uv,
            "--no-config",
            "pip",
            "install",
            "--python",
            python,
            "--require-hashes",
            "--no-deps",
            "--only-binary",
            ":all:",
            "-r",
            build_lock,
        ]
    )
    env = dict(
        os.environ, SOURCE_DATE_EPOCH="1722470400", UV_NO_CONFIG="1", UV_OFFLINE="1"
    )
    if platform.system() == "Darwin":
        env["MACOSX_DEPLOYMENT_TARGET"] = "14.0"
    bundled = []
    requirements = [
        "# Generated by scripts/separation_runtime/prepare.py. No dependency resolution."
    ]
    for artifact in artifacts["dependencies"]:
        if artifact.get("bundled"):
            source = download(artifact["source"], cache)
            output = work / ("built-" + artifact["name"])
            if output.exists():
                shutil.rmtree(output)
            output.mkdir()
            run(
                [
                    uv,
                    "--no-config",
                    "build",
                    "--wheel",
                    "--no-build-isolation",
                    "--offline",
                    "--python",
                    python,
                    "--out-dir",
                    output,
                    source,
                ],
                env=env,
            )
            built = list(output.glob("*.whl"))
            if len(built) != 1:
                raise ValueError(f"Expected exactly one wheel for {artifact['name']}")
            wheel = wheels / built[0].name
            shutil.copyfile(built[0], wheel)
            sha = digest(wheel)
            bundled.append(
                {
                    "name": artifact["name"],
                    "version": artifact["version"],
                    "filename": wheel.name,
                    "sha256": sha,
                    "bytes": wheel.stat().st_size,
                    "source": artifact["source"],
                }
            )
            url = "${RESOURCE_DIR}/wheels/" + wheel.name
            # Wheel archives carry licenses; also expose them beside bootstrap notices.
            with zipfile.ZipFile(wheel) as archive:
                for name in archive.namelist():
                    if "license" in name.lower():
                        (
                            notices / (artifact["name"] + "-" + name.replace("/", "-"))
                        ).write_bytes(archive.read(name))
        else:
            sha, url = artifact["sha256"], artifact["url"]
        requirements.append(f"{artifact['name']} @ {url} --hash=sha256:{sha}")
    (resource / "requirements.lock").write_text("\n".join(requirements) + "\n")
    inventory = {
        "schema_version": 1,
        "target": target,
        "bundled_wheels": bundled,
        "tools": [
            {
                "path": str(path.relative_to(resource)),
                "sha256": digest(path),
                "bytes": path.stat().st_size,
            }
            for path in (uv, ffmpeg)
        ],
        "ffmpeg_build": subprocess.check_output([ffmpeg, "-version"], text=True),
    }
    (resource / "bundled-artifacts.json").write_text(
        json.dumps(inventory, indent=2) + "\n"
    )
    print(
        f"Prepared {target}: {len(requirements)-1} locked inference distributions, {len(bundled)} bundled wheels"
    )


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--build-python",
        default=shutil.which("python3") or "python",
        help="Maintainer Python 3.12 executable",
    )
    args = parser.parse_args()
    target = TARGETS.get((platform.system(), platform.machine()))
    if not target:
        parser.error("No separation runtime is defined for this build host")
    version = subprocess.check_output(
        [
            args.build_python,
            "-c",
            "import sys; print('.'.join(map(str,sys.version_info[:2])))",
        ],
        text=True,
    ).strip()
    if version != "3.12":
        parser.error("The native wheel build requires Python 3.12")
    prepare(target, args.build_python)


if __name__ == "__main__":
    main()
