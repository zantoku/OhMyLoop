# Stem prototype data cleanup

Inventory checked: 2026-10-14. No data was removed while preparing these instructions.
The integrated app will download its own verified model, without importing the
prototype weights. **Keep the development Python runtime for issue #6.**

Close the listening harness and stop prototype inference/setup before cleanup.
Close OhMyLoop before removing temporary PCM. Use Finder's Move to Trash for the
specific directories below, then empty Trash when satisfied they are no longer
needed. Sizes are local `du` estimates; shared files can change reclaimed space.

For scripted cleanup, run from the checkout:

```sh
# Preview old prototype weights, caches and setup scratch:
bash scripts/cleanup-stem-prototype.sh
# Permanently remove the listed directories:
bash scripts/cleanup-stem-prototype.sh --apply
# Also remove old prototype results/mixes (preview first):
bash scripts/cleanup-stem-prototype.sh --include-results
bash scripts/cleanup-stem-prototype.sh --include-results --apply
```

The script preserves the development Python runtime, the managed library, original
recordings outside the prototype results, and the integrated app's model download.
It refuses symlinked cleanup paths. It does not scan app data or temporary directories
for crash leftovers. Deletion is permanent; use Finder instead if you prefer Trash.

## Removable prototype data

Paths are relative to the repository root.

| Directory | Current size | Effect of removal |
| --- | ---: | --- |
| `.tools/stems/models/roformer-sw` | 667 MiB | Removes the prototype Roformer weights/config/lock; prototype inference needs another download. |
| `.tools/stems/models/demucs-6s` | 52 MiB | Removes the old comparison model; Demucs prototype inference becomes unavailable. |
| `.tools/stems/uv-cache` | 930 MiB | Removes the package acquisition cache; later developer setup may download packages again. |
| `.tools/stems/tmp` contents | 4 KiB | Removes setup scratch while setup/jobs are idle. |
| `artifacts/stem-prototype` | 3.8 GiB | Removes prototype separation results and mixes; standalone comparisons lose their inputs. |

You can remove individual result directories instead of the entire result collection:
`bones-demucs-6s` is about 698 MiB and `bones-roformer-sw` about 798 MiB.
There are other completed comparisons in that collection. Removing these results
is optional: keep any you still want to audition or reproduce.

Stems attached through the managed-library UI are copied into the library. Their
playback does not depend on these prototype output directories or old model files.
Preserve any outputs being used outside that managed workflow until no longer needed.

The prototype also configures `.tools/stems/torch`, `.tools/stems/hf`,
`.tools/stems/cache` and `.tools/stems/numba` as cache locations. None existed at
inventory time. If they appear later, they are disposable caches while jobs are
idle; removing them can require rebuilding/downloading cache contents next time.

## Keep

- `.tools/stems/venv` — about 883 MiB, the selected #6 development environment.
- `.tools/stems/python` — about 72 MiB, the Python installation that environment
  points into.
- The managed library, its song documents, recordings, associated stems and
  `playback-cache` generations.
- Your original recordings, including `BONES.m4a`.

**Do not delete all of `.tools/stems` or all of `artifacts`.** Those broader paths
can contain required runtime tools, original recordings or unrelated assets.

## Temporary playback PCM

Native harness/preparation scratch uses directories named `ohmyloop-stems-<UUID>`
under the system temporary directory (`$TMPDIR`). Normal cleanup occurs when the
last Rust owner releases the files; abrupt termination can leave scratch behind.
Only remove these exact scratch directories when the app/harness and jobs are
closed. Do not purge the general temporary directory.

At inventory time one matching directory existed and was empty. No incomplete
prototype jobs or model downloads were present. There is no large identified
temporary-PCM cleanup waiting to reclaim space.

## Integrated app data after #6

Use **Stems → Model management → Remove model** for the app's downloaded weights.
Completed stems keep playing without that model. Use confirmed **Remove stems**
for a song's associated stems/cache, or confirmed song deletion for that song.

Aborted private work normally cleans itself up. Crash leftovers and cleanup-failure
paths need manual attention; #6 intentionally provides no automatic orphan scanner,
journal or recovery UI. Do not delete arbitrary directories inside a managed
library. The #6 usage guide must document the implemented app-data location and
how to identify a specific reported uncommitted directory without touching the
set referenced by `song.json`.

## Integrated #6 locations

The Stems screen's **Model management** displays the exact shared model path.
The normal macOS location is
`~/Library/Application Support/org.ohmyloop.desktop/models/separation/roformer-sw-v1`.
Use **Remove model** while idle to remove these downloaded assets; existing stems
remain playable. Failed/cancelled partial downloads are removed immediately; if
abrupt termination left `.partial` files, quit the app before removing those exact
files in the displayed model directory. Completed verified files can remain for retry.
Do not remove the development `.tools/stems/venv` or managed `.tools/stems/python`
unless retiring the runtime as well.

A generated attempt uses `songs/<song UUID>/stems/<set UUID>` in the managed library.
Normal failures/cancellation remove it; no restart scanner is provided. To clean
crash leftovers manually, quit the app, inspect that song's `song.json`, and keep
its current `stems.id` directory. Remove only a separately identified unassociated
attempt directory; never remove the whole `songs`, `stems`, or `playback-cache` tree
by assuming every UUID directory is scratch. The active set owns its WAVs and PCM
cache. Diagnostic cleanup errors include the exact uncommitted path when available.
