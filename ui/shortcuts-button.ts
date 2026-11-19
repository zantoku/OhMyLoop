// Whether Practice shows the header's keyboard-shortcuts button (Settings F3).
// Per computer, like the theme; the list stays reachable from Settings and
// from its shortcut either way.
import { readChoice, writeChoice } from './stored-choice';

const storageKey = 'ohmyloop.ui.practice-shortcuts-button';
const choices = ['shown', 'hidden'] as const;

export function readShortcutsButton(): boolean {
  return readChoice(storageKey, choices, 'shown') === 'shown';
}

/** False when the choice couldn't be saved for next time. */
export function saveShortcutsButton(shown: boolean): boolean {
  return writeChoice(storageKey, shown ? 'shown' : 'hidden');
}
