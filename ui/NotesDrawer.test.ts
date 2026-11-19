import { afterEach, describe, expect, it, vi } from 'vitest';
import { cleanup, fireEvent, render, screen } from '@testing-library/svelte';
import NotesDrawer from './NotesDrawer.svelte';
import { Keymap } from './keymap';
import { ownsKey } from './key-routing';
import type { NotesState } from './notes-session';

const calm: NotesState = { text: 'Bend lick: hit the target pitch', dirty: false, pending: false, slow: false, tooLong: false, error: '' };
function drawer(notes: Partial<NotesState> = {}, font: 'proportional' | 'monospace' = 'proportional') {
  const home = document.createElement('div'); home.tabIndex = -1; home.dataset.home = ''; document.body.append(home);
  const run = vi.fn(), onchange = vi.fn();
  render(NotesDrawer, { notes: { ...calm, ...notes }, font, keymap: new Keymap(), available: () => true, onchange, run });
  return { home, run, onchange, field: screen.getByRole('textbox', { name: 'Notes for this song' }) as HTMLTextAreaElement };
}

describe('notes drawer', () => {
  afterEach(() => { cleanup(); document.body.replaceChildren(); });

  it('takes focus for writing, keeps plain text exactly and reports every edit', async () => {
    const { field, onchange } = drawer();
    expect(document.activeElement).toBe(field);
    expect(field.value).toBe(calm.text);
    expect(field.getAttribute('spellcheck')).toBe('true');
    await fireEvent.input(field, { target: { value: '  m. 33\n\n  run starts on "and"  ' } });
    expect(onchange).toHaveBeenCalledWith('  m. 33\n\n  run starts on "and"  ');
    expect(screen.queryByRole('status')).toBeNull();
    expect(screen.queryByRole('alert')).toBeNull();
  });

  it('owns every key while typing; Esc returns home so the shortcuts work again', async () => {
    const { field, home } = drawer();
    for (const key of [' ', 'n', 's', 'l', 'a', '1', 'Enter', 'ArrowLeft', 'Escape']) expect(ownsKey(field, key, 'keyboard'), key).toBe(true);
    expect(screen.getByText('Typing · Esc returns to the waveform, so Space plays again')).toBeTruthy();
    const escape = new KeyboardEvent('keydown', { key: 'Escape', bubbles: true, cancelable: true });
    field.dispatchEvent(escape);
    expect(escape.defaultPrevented).toBe(true);
    expect(document.activeElement).toBe(home);
    // At home, nothing in the drawer owns the keys: they reach the dispatcher.
    for (const key of [' ', 'n', 'l', 'Escape']) expect(ownsKey(home, key, 'pointer'), key).toBe(false);
    await fireEvent.blur(field);
    expect(screen.getByText('Saved as you type')).toBeTruthy();
  });

  it('leaves Esc alone while an input method is composing', () => {
    const { field } = drawer();
    const escape = new KeyboardEvent('keydown', { key: 'Escape', bubbles: true, cancelable: true, isComposing: true });
    field.dispatchEvent(escape);
    expect(escape.defaultPrevented).toBe(false);
    expect(document.activeElement).toBe(field);
  });

  it('lets its Retry button own only Space and Enter, and only from the keyboard', () => {
    drawer({ error: 'Error: Disk full', dirty: true });
    const retry = screen.getByRole('button', { name: 'Retry save' });
    for (const key of [' ', 'Enter']) expect(ownsKey(retry, key, 'keyboard'), key).toBe(true);
    for (const key of [' ', 'Enter']) expect(ownsKey(retry, key, 'pointer'), key).toBe(false);
    for (const key of ['n', 'l', 'Escape', 'ArrowLeft']) expect(ownsKey(retry, key, 'keyboard'), key).toBe(false);
  });

  it('shows Saving… only for a slow save', () => {
    drawer({ pending: true, dirty: true });
    expect(screen.queryByText('Saving…')).toBeNull();
    cleanup(); document.body.replaceChildren();
    drawer({ pending: true, slow: true, dirty: true });
    expect(screen.getByRole('status').textContent).toBe('Saving…');
  });

  it('explains a failed save, keeps the text and offers Retry', async () => {
    const { field, run } = drawer({ text: 'Do not lose this', dirty: true, error: 'Error: Disk full.' });
    expect(screen.getByRole('alert').textContent).toContain('Couldn’t save your notes: Disk full. Your text is kept here.');
    expect(field.value).toBe('Do not lose this');
    await fireEvent.click(screen.getByRole('button', { name: 'Retry save' }));
    expect(run).toHaveBeenCalledWith('retry_notes');
  });

  it('explains over-long notes without offering Retry', () => {
    const { field } = drawer({ dirty: true, tooLong: true });
    field.blur();
    expect(screen.queryByText('Saved as you type')).toBeNull();
    expect(screen.getByRole('alert').textContent).toContain('Notes are over 1 MiB and can’t be saved. The last saved version is kept; shorten them to save again.');
    expect(screen.queryByRole('button', { name: 'Retry save' })).toBeNull();
  });

  it('reads the notes font preference', () => {
    expect(drawer({}, 'proportional').field.classList.contains('mono')).toBe(false);
    cleanup(); document.body.replaceChildren();
    expect(drawer({}, 'monospace').field.classList.contains('mono')).toBe(true);
  });

  it('returns focus to where it was when it closes', () => {
    const before = document.createElement('button'); document.body.append(before); before.focus();
    const { field, home } = drawer();
    expect(document.activeElement).toBe(field);
    cleanup();
    expect(document.activeElement).toBe(before);
    before.remove(); home.remove();
  });

  it('is read-only when asked, for while the app is busy', () => {
    render(NotesDrawer, { notes: calm, readonly: true, font: 'proportional', keymap: new Keymap(), available: () => true, onchange: vi.fn(), run: vi.fn() });
    expect((screen.getByRole('textbox', { name: 'Notes for this song' }) as HTMLTextAreaElement).readOnly).toBe(true);
  });

  it('returns focus home when the previous element is hidden or inert by the time it closes', () => {
    for (const attribute of ['hidden', 'inert']) {
      const pane = document.createElement('div'); const before = document.createElement('button'); pane.append(before); document.body.append(pane); before.focus();
      const { home } = drawer();
      pane.setAttribute(attribute, '');
      cleanup();
      expect(document.activeElement, attribute).toBe(home);
      document.body.replaceChildren();
    }
  });
});
