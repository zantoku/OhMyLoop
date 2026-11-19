// Undo and Redo go to the focused text field when there is one, otherwise to
// the current screen's app undo. Shared by the menu bar and the webview handler.
import type { ActionId, ActionScope } from './actions';
import { isTextEntry } from './key-routing';

export type EditCommand = 'undo' | 'redo';
const APP_UNDO: Partial<Record<ActionScope, Partial<Record<EditCommand, ActionId>>>> = {
  practice: { undo: 'undo_delete' },
  mapping: { undo: 'map_undo', redo: 'map_redo' }
};

/** The app action that undoes or redoes on `screen`, if it has one. */
export function appEditAction(screen: ActionScope, command: EditCommand): ActionId | null {
  return APP_UNDO[screen]?.[command] ?? null;
}

/** ⌘Z / ⇧⌘Z on macOS, Ctrl+Z / Ctrl+Shift+Z elsewhere. */
export function editCommand(event: Pick<KeyboardEvent, 'key' | 'shiftKey' | 'ctrlKey' | 'metaKey' | 'altKey'>, mac: boolean): EditCommand | null {
  if (event.key.toLowerCase() !== 'z' || event.altKey) return null;
  const modifier = mac ? event.metaKey && !event.ctrlKey : event.ctrlKey && !event.metaKey;
  return modifier ? event.shiftKey ? 'redo' : 'undo' : null;
}

/** Menu Undo/Redo: the focused text field's own history first, otherwise the app's. */
export function routeEditCommand(command: EditCommand, doc: Document, app: (command: EditCommand) => void): void {
  const active = doc.activeElement;
  if (active instanceof Element && isTextEntry(active)) doc.execCommand(command);
  else app(command);
}
