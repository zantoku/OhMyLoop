// Per-computer UI preferences: theme, notes font and the Practice shortcuts
// button. Each validates its stored value and falls back to the default
// without overwriting it.
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { readThemePreference, saveThemePreference } from './theme';
import { readNotesFont, saveNotesFont } from './notes-font';
import { readShortcutsButton, saveShortcutsButton } from './shortcuts-button';

beforeEach(() => { localStorage.clear(); delete document.documentElement.dataset.theme; });

describe('theme', () => {
  it('defaults to auto, applies at once and persists', () => {
    expect(readThemePreference()).toBe('auto');
    expect(saveThemePreference('dark')).toBe(true);
    expect(document.documentElement.dataset.theme).toBe('dark');
    expect(readThemePreference()).toBe('dark');
  });
  it('falls back to auto without overwriting an unknown stored value', () => {
    localStorage.setItem('ohmyloop.ui.theme', 'future-theme');
    expect(readThemePreference()).toBe('auto');
    expect(localStorage.getItem('ohmyloop.ui.theme')).toBe('future-theme');
  });
  it('still applies when it can’t be saved, and reads auto when storage is blocked', () => {
    const write = vi.spyOn(Storage.prototype, 'setItem').mockImplementation(() => { throw new Error('Storage unavailable'); });
    expect(saveThemePreference('light')).toBe(false);
    expect(document.documentElement.dataset.theme).toBe('light');
    write.mockRestore();
    const read = vi.spyOn(Storage.prototype, 'getItem').mockImplementation(() => { throw new Error('Storage unavailable'); });
    expect(readThemePreference()).toBe('auto');
    read.mockRestore();
  });
});

describe('notes font writer', () => {
  it('saves a choice the reader returns, and reports a failed save', () => {
    expect(saveNotesFont('monospace')).toBe(true);
    expect(readNotesFont()).toBe('monospace');
    const write = vi.spyOn(Storage.prototype, 'setItem').mockImplementation(() => { throw new Error('full'); });
    expect(saveNotesFont('proportional')).toBe(false);
    write.mockRestore();
  });
});

describe('Practice shortcuts button', () => {
  it('shows by default, persists hiding, and ignores an unknown value without overwriting it', () => {
    expect(readShortcutsButton()).toBe(true);
    expect(saveShortcutsButton(false)).toBe(true);
    expect(readShortcutsButton()).toBe(false);
    localStorage.setItem('ohmyloop.ui.practice-shortcuts-button', 'sometimes');
    expect(readShortcutsButton()).toBe(true);
    expect(localStorage.getItem('ohmyloop.ui.practice-shortcuts-button')).toBe('sometimes');
  });
});
