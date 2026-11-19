// The notes font preference: the proportional UI font by default, or monospace
// so chord charts and tab line up (Notes decision 5, Settings decision 5).
// Settings → Appearance will add the writer; until then the drawer only reads it.
export const notesFonts = ['proportional', 'monospace'] as const;
export type NotesFont = typeof notesFonts[number];

const storageKey = 'ohmyloop.ui.notes-font';

export function readNotesFont(): NotesFont {
  try {
    const value = localStorage.getItem(storageKey);
    return notesFonts.find(font => font === value) ?? 'proportional';
  } catch {
    return 'proportional';
  }
}
