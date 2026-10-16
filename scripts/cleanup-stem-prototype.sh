#!/usr/bin/env bash
# SPDX-License-Identifier: GPL-3.0-or-later
set -euo pipefail

usage() {
  cat <<'EOF'
Usage: scripts/cleanup-stem-prototype.sh [--apply] [--include-results]

Preview removal of old prototype models, download caches and setup scratch.
  --apply            Permanently delete the listed directories.
  --include-results  Also remove artifacts/stem-prototype (old separated audio/mixes).
  -h, --help         Show this help.

Close OhMyLoop, the listening harness and prototype/setup jobs before applying.
Keeps the Python runtime, managed library and integrated app's downloaded model.
No files are deleted without --apply. There is no undo.
EOF
}

apply=false
include_results=false
for arg in "$@"; do
  case "$arg" in
    --apply) apply=true ;;
    --include-results) include_results=true ;;
    -h|--help) usage; exit 0 ;;
    *) printf 'Unknown option: %s\n' "$arg" >&2; usage >&2; exit 2 ;;
  esac
done

repo_root=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd -P)
if [[ ! -f "$repo_root/Cargo.toml" || ! -f "$repo_root/scripts/stem_prototype/requirements.lock" ]]; then
  printf 'Cannot identify the OhMyLoop checkout; refusing cleanup.\n' >&2
  exit 1
fi

targets=(
  .tools/stems/models/roformer-sw
  .tools/stems/models/demucs-6s
  .tools/stems/uv-cache
  .tools/stems/torch
  .tools/stems/hf
  .tools/stems/cache
  .tools/stems/numba
  .tools/stems/tmp
)
if "$include_results"; then
  targets+=(artifacts/stem-prototype)
fi

# Validate every target before deleting anything. Refuse symlinked ancestors,
# including a symlink at the target, so cleanup cannot escape the checkout.
for relative in "${targets[@]}"; do
  current=$repo_root
  IFS=/ read -r -a components <<< "$relative"
  for component in "${components[@]}"; do
    current="$current/$component"
    if [[ -L "$current" ]]; then
      printf 'Refusing cleanup through symlink: %s\n' "$current" >&2
      exit 1
    fi
  done
  if [[ -e "$current" && ! -d "$current" ]]; then
    printf 'Expected a directory; refusing cleanup: %s\n' "$current" >&2
    exit 1
  fi
done

printf 'Checkout: %s\n' "$repo_root"
found=false
for relative in "${targets[@]}"; do
  target="$repo_root/$relative"
  [[ -d "$target" ]] || continue
  found=true
  du -sh -- "$target"
  if "$apply"; then
    rm -rf -- "$target"
    printf 'Removed: %s\n' "$relative"
  fi
done

if ! "$found"; then
  printf 'Nothing to remove.\n'
elif ! "$apply"; then
  printf '\nPreview only. Run again with --apply to delete these directories.\n'
fi
