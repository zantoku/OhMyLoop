// The notes font preference: the proportional UI font by default, or monospace
// so chord charts and tab line up (Notes decision 5, Settings decision 5).
// Settings → Appearance writes it; the notes drawer reads it.
import { readChoice, writeChoice } from './stored-choice';

export const notesFonts = ['proportional', 'monospace'] as const;
export type NotesFont = typeof notesFonts[number];

const storageKey = 'ohmyloop.ui.notes-font';

export function readNotesFont(): NotesFont {
  return readChoice(storageKey, notesFonts, 'proportional');
}

/** False when the choice couldn't be saved for next time. */
export function saveNotesFont(font: NotesFont): boolean {
  return writeChoice(storageKey, font);
}
