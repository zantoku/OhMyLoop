// Importing by dropping a file onto the window (library.md, decision 9):
// exactly one file goes through the same flow as Import; anything else is
// refused with a short explanation. Rust checks the file type.
import type { FileDrop } from './bridge';
import { fileName } from './library';

/** What a drag currently shows: the file it would import, or why it can't. */
export type DropPreview = { name: string } | { refused: string };

/** Why these dropped paths can't be imported, or null for a single file. */
export function dropRefusal(paths: readonly string[]): string | null {
  if (paths.length === 1) return null;
  return paths.length ? `Drop one audio file at a time. ${paths.length} files were dropped; importing several files or a folder isn’t supported yet.` : 'Drop an audio file to import it.';
}

export function dropPreview(paths: readonly string[]): DropPreview {
  const refused = dropRefusal(paths);
  return refused ? { refused: 'Drop one audio file at a time' } : { name: fileName(paths[0]) };
}

/**
 * Follows a drag over the window. `accepts` says whether an import may start
 * now; a drag that arrives while it can't is ignored, without an overlay.
 */
export class FileDropTarget {
  private paths: readonly string[] | null = null;
  constructor(private accepts: () => boolean, private show: (preview: DropPreview | null) => void,
    private importFile: (path: string) => void, private refuse: (reason: string) => void) {}

  handle(event: FileDrop): void {
    if (event.type === 'enter') {
      this.paths = this.accepts() ? event.paths : null;
      this.show(this.paths ? dropPreview(this.paths) : null);
    } else if (event.type === 'leave') {
      this.paths = null; this.show(null);
    } else if (event.type === 'drop') {
      const accepted = this.paths !== null && this.accepts();
      this.paths = null; this.show(null);
      if (!accepted) return;
      const refused = dropRefusal(event.paths);
      if (refused) this.refuse(refused); else this.importFile(event.paths[0]);
    }
  }
}
