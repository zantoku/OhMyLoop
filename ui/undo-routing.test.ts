import { afterEach, describe, expect, it, vi } from 'vitest';
import { appEditAction, editCommand, routeEditCommand } from './undo-routing';

afterEach(() => { document.body.innerHTML = ''; });
const key = (key: string, modifiers: Partial<Record<'shiftKey' | 'ctrlKey' | 'metaKey' | 'altKey', boolean>> = {}) =>
  ({ key, shiftKey: false, ctrlKey: false, metaKey: false, altKey: false, ...modifiers });

describe('undo routing', () => {
  it('recognises ⌘Z/⇧⌘Z on macOS and Ctrl+Z/Ctrl+Shift+Z elsewhere', () => {
    expect(editCommand(key('z', { metaKey: true }), true)).toBe('undo');
    expect(editCommand(key('Z', { metaKey: true, shiftKey: true }), true)).toBe('redo');
    expect(editCommand(key('z', { ctrlKey: true }), true)).toBeNull();
    expect(editCommand(key('z', { ctrlKey: true }), false)).toBe('undo');
    expect(editCommand(key('z', { ctrlKey: true, shiftKey: true }), false)).toBe('redo');
    expect(editCommand(key('z'), false)).toBeNull();
    expect(editCommand(key('z', { ctrlKey: true, altKey: true }), false)).toBeNull();
  });

  it('maps each screen to its app undo', () => {
    expect(appEditAction('practice', 'undo')).toBe('undo_delete');
    expect(appEditAction('practice', 'redo')).toBeNull();
    expect(appEditAction('mapping', 'undo')).toBe('map_undo');
    expect(appEditAction('mapping', 'redo')).toBe('map_redo');
    expect(appEditAction('library', 'undo')).toBeNull();
  });

  it('sends Undo/Redo to a focused text field, otherwise to the app', () => {
    const exec = vi.fn(() => true);
    Object.defineProperty(document, 'execCommand', { value: exec, configurable: true });
    const app = vi.fn();
    document.body.innerHTML = '<input><button>Play</button>';
    document.querySelector('input')!.focus();
    routeEditCommand('undo', document, app);
    expect(exec).toHaveBeenCalledWith('undo'); expect(app).not.toHaveBeenCalled();
    document.querySelector('input')!.blur();
    routeEditCommand('redo', document, app);
    expect(app).toHaveBeenCalledWith('redo'); expect(exec).toHaveBeenCalledOnce();
  });
});
