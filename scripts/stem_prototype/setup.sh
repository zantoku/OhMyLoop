#!/usr/bin/env bash
# SPDX-License-Identifier: GPL-3.0-or-later
set -euo pipefail
cd "$(dirname "$0")/../.."
export UV_CACHE_DIR="$PWD/.tools/stems/uv-cache"
export UV_PYTHON_INSTALL_DIR="$PWD/.tools/stems/python"
export TMPDIR="$PWD/.tools/stems/tmp"
mkdir -p "$TMPDIR"
uv python install --no-bin --no-registry 3.12.13
uv venv --allow-existing --python 3.12.13 --managed-python .tools/stems/venv
uv pip install --python .tools/stems/venv/bin/python -r scripts/stem_prototype/requirements.lock
uv pip check --python .tools/stems/venv/bin/python
