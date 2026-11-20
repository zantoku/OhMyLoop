import { beforeEach, expect, it, vi } from 'vitest';
import { readNotesFont } from './notes-font';

beforeEach(() => localStorage.clear());

it('defaults the notes font to proportional and reads a saved choice', () => {
  expect(readNotesFont()).toBe('proportional');
  localStorage.setItem('ohmyloop.ui.notes-font', 'monospace');
  expect(readNotesFont()).toBe('monospace');
});

it('ignores an invalid stored value without overwriting it, and survives blocked storage', () => {
  localStorage.setItem('ohmyloop.ui.notes-font', 'comic');
  expect(readNotesFont()).toBe('proportional');
  expect(localStorage.getItem('ohmyloop.ui.notes-font')).toBe('comic');
  const blocked = vi.spyOn(Storage.prototype, 'getItem').mockImplementation(() => { throw new Error('blocked'); });
  expect(readNotesFont()).toBe('proportional');
  blocked.mockRestore();
});
