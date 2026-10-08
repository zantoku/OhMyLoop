#!/usr/bin/env python3
"""Verify every bundled Rubber Band file against its pinned upstream Git blob."""
import hashlib
import json
from pathlib import Path

root = Path(__file__).resolve().parents[1] / "vendor" / "rubberband"
manifest = json.loads((root / "UPSTREAM.json").read_text())
failures = []
for item in manifest["files"]:
    data = (root / item["path"]).read_bytes()
    blob = b"blob " + str(len(data)).encode() + b"\0" + data
    if hashlib.sha1(blob).hexdigest() != item["sha"]:
        failures.append(item["path"])
if failures:
    raise SystemExit("Modified vendor files: " + ", ".join(failures))
print(f"Verified {len(manifest['files'])} files at {manifest['commit']}")

