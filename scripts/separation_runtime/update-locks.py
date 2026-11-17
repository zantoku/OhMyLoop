#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-or-later
"""Rebuild reviewed artifact metadata, never run on end-user machines.

Requires Python 3.12 and packaging (the prototype environment supplies both).
This explicitly refreshes pinned upstream metadata; review its diff and rebuild
all supported target resources after changes. It never chooses a newer version.
"""
import json, pathlib, urllib.request, html, re
from packaging.tags import cpython_tags, compatible_tags, mac_platforms
from packaging.utils import parse_wheel_filename

ROOT = pathlib.Path(__file__).resolve().parents[2]
root = ROOT / "resources/separation/runtime"
meta = ROOT / ".tools/runtime-packaging/metadata"
meta.mkdir(parents=True, exist_ok=True)
pins = dict(
    line.strip().split("==")
    for line in open(ROOT / "scripts/stem_prototype/requirements.lock")
)


def fetch_json(url, path):
    with urllib.request.urlopen(url, timeout=60) as response:
        path.write_bytes(response.read())


fetch_json(
    "https://raw.githubusercontent.com/astral-sh/uv/0.12.16/crates/uv-python/download-metadata.json",
    meta / "python-downloads.json",
)
fetch_json(
    "https://api.github.com/repos/astral-sh/uv/releases/tags/0.12.16",
    meta / "uv-release.json",
)
fetch_json(
    "https://api.github.com/repos/astral-sh/python-build-standalone/releases/tags/20260807",
    meta / "python-release.json",
)
for name, version in [
    *pins.items(),
    ("imageio-ffmpeg", "0.6.0"),
    ("diffq-fixed", "0.2.4"),
]:
    fetch_json(f"https://pypi.org/pypi/{name}/{version}/json", meta / f"{name}.json")
pydata = json.load(open(meta / "python-downloads.json"))
uv = json.load(open(meta / "uv-release.json"))
pyrelease = json.load(open(meta / "python-release.json"))
for name, version in [("colorama", "0.4.6")]:
    with urllib.request.urlopen(f"https://pypi.org/pypi/{name}/{version}/json") as f:
        (meta / f"{name}.json").write_bytes(f.read())
for name in ["torch", "torchaudio", "torchvision"]:
    with urllib.request.urlopen(f"https://download.pytorch.org/whl/cpu/{name}/") as f:
        (meta / f"{name}-cpu.html").write_bytes(f.read())
for triple, os, arch, libc in [
    ("aarch64-apple-darwin", "darwin", "aarch64", "none"),
    ("x86_64-unknown-linux-gnu", "linux", "x86_64", "gnu"),
    ("x86_64-pc-windows-msvc", "windows", "x86_64", "none"),
]:
    d = root / triple
    d.mkdir(parents=True, exist_ok=True)
    platforms = (
        list(mac_platforms((14, 0), "arm64"))
        if os == "darwin"
        else (
            ["win_amd64"]
            if os == "windows"
            else [f"manylinux_2_{v}_x86_64" for v in range(28, 4, -1)]
            + [
                "manylinux2014_x86_64",
                "manylinux2010_x86_64",
                "manylinux1_x86_64",
                "linux_x86_64",
            ]
        )
    )
    taglist = list(cpython_tags((3, 12), platforms=platforms)) + list(
        compatible_tags((3, 12), interpreter="cp312", platforms=platforms)
    )
    rank = {tag: i for i, tag in reversed(list(enumerate(taglist)))}
    selected = []
    targetpins = dict(pins)
    if os == "windows":
        del targetpins["diffq"]
        targetpins.update({"diffq-fixed": "0.2.4", "colorama": "0.4.6"})
    for n, v in targetpins.items():
        data = json.load(open(meta / f"{n}.json"))
        if n in ["torch", "torchaudio", "torchvision"] and os != "darwin":
            candidates = []
            for link in re.findall(
                r'href="([^"]+)"', open(meta / f"{n}-cpu.html").read()
            ):
                link = html.unescape(link)
                url, _, frag = link.partition("#")
                filename = urllib.parse.unquote(url.rsplit("/", 1)[-1])
                filename = filename.split("?")[0]
                if not filename.endswith(".whl"):
                    continue
                _, ver, _, tags = parse_wheel_filename(filename)
                if str(ver) != v + "+cpu" or not set(tags).intersection(rank):
                    continue
                candidates.append(
                    (
                        min(rank[t] for t in tags if t in rank),
                        {
                            "name": n,
                            "version": v + "+cpu",
                            "filename": filename,
                            "url": url,
                            "sha256": frag.split("=")[-1],
                        },
                    )
                )
            chosen = min(candidates, key=lambda x: x[0])[1]
            req = urllib.request.Request(chosen["url"], method="HEAD")
            with urllib.request.urlopen(req) as f:
                chosen["bytes"] = int(f.headers.get("Content-Length", 0))
        else:
            candidates = []
            for u in data["urls"]:
                if u["filename"].endswith(".whl"):
                    tags = parse_wheel_filename(u["filename"])[3]
                    compatible = tags.intersection(rank)
                    if compatible:
                        candidates.append((min(rank[t] for t in compatible), u))
            if candidates:
                u = min(candidates, key=lambda x: x[0])[1]
                chosen = {
                    "name": n,
                    "version": v,
                    "filename": u["filename"],
                    "url": u["url"],
                    "sha256": u["digests"]["sha256"],
                    "bytes": u["size"],
                }
            else:
                u = next(u for u in data["urls"] if u["packagetype"] == "sdist")
                chosen = {
                    "name": n,
                    "version": v,
                    "source": {
                        "filename": u["filename"],
                        "url": u["url"],
                        "sha256": u["digests"]["sha256"],
                        "bytes": u["size"],
                    },
                    "bundled": True,
                }
        selected.append(chosen)
    key = f"cpython-3.12.13-{os}-{arch}-{libc}"
    (d / "python-downloads.json").write_text(
        json.dumps({key: pydata[key]}, indent=2) + "\n"
    )
    uvname = f"uv-{triple}." + ("zip" if os == "windows" else "tar.gz")
    u = next(x for x in uv["assets"] if x["name"] == uvname)
    ffdata = json.load(open(meta / "imageio-ffmpeg.json"))
    ffcandidates = []
    for u2 in ffdata["urls"]:
        if not u2["filename"].endswith(".whl"):
            continue
        compatible = parse_wheel_filename(u2["filename"])[3].intersection(rank)
        if compatible:
            ffcandidates.append((min(rank[t] for t in compatible), u2))
    ff = min(ffcandidates, key=lambda x: x[0])[1]
    artifacts = {
        "schema_version": 1,
        "uv": {
            "version": "0.12.16",
            "filename": u["name"],
            "url": u["browser_download_url"],
            "sha256": u["digest"].split(":")[1],
            "bytes": u["size"],
        },
        "ffmpeg": {
            "provider": "imageio-ffmpeg",
            "version": "0.6.0",
            "filename": ff["filename"],
            "url": ff["url"],
            "sha256": ff["digests"]["sha256"],
            "bytes": ff["size"],
        },
        "dependencies": selected,
    }
    py = pydata[key]
    pa = next(
        a
        for a in pyrelease["assets"]
        if a["name"] == urllib.parse.unquote(py["url"].rsplit("/", 1)[-1])
    )
    artifacts["python"] = {
        "filename": pa["name"],
        "url": py["url"],
        "sha256": py["sha256"],
        "bytes": pa["size"],
    }
    (d / "artifacts.json").write_text(json.dumps(artifacts, indent=2) + "\n")
    definition = {
        "schema_version": 1,
        "id": f"roformer-sw-cpu2-v1-{triple}",
        "target": triple,
        "python_version": "3.12.13",
        "python_key": key,
        "python_installation": key.replace("-darwin-", "-macos-"),
        "python_build": "20260807",
        "python_executable": "python.exe" if os == "windows" else "bin/python3",
        "python_downloads": "python-downloads.json",
        "requirements": "requirements.lock",
        "uv": "tools/uv" + (".exe" if os == "windows" else ""),
        "ffmpeg": "tools/ffmpeg" + (".exe" if os == "windows" else ""),
        "uv_version": "0.12.16",
        "ffmpeg_version": "7.0.2" if os == "linux" else "7.1",
        "artifact_manifest": "artifacts.json",
        "minimum_os": (
            {"family": "macos", "version": "14.0"}
            if os == "darwin"
            else (
                {"family": "glibc", "version": "2.28"}
                if os == "linux"
                else {"family": "windows", "version": "10.0"}
            )
        ),
        "download_bytes": sum(
            a.get("bytes", a.get("source", {}).get("bytes", 0)) for a in selected
        )
        + 40_000_000,
        "installed_bytes": 3_000_000_000 if os == "windows" else 1_500_000_000,
        "temporary_bytes": 5_000_000_000 if os == "windows" else 2_500_000_000,
    }
    definition["download_bytes"] = artifacts["python"]["bytes"] + sum(
        a.get("bytes", 0) for a in selected if not a.get("bundled")
    )
    (d / "definition.json").write_text(json.dumps(definition, indent=2) + "\n")
    print(
        triple,
        "bundled",
        [a["name"] for a in selected if a.get("bundled")],
        "bytes",
        definition["download_bytes"],
    )
