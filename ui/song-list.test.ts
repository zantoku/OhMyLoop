import { describe, expect, it } from 'vitest';
import { moveSelection, SONG_LIST_KEYS } from './song-list';
import { ownsKey } from './key-routing';

const keys = ['a', 'b', 'c', 'd', 'e'];
describe('song list selection', () => {
  it('moves with the arrows, pages and ends, stopping at the edges', () => {
    expect(moveSelection(keys, 'b', 'ArrowDown', 3)).toBe('c');
    expect(moveSelection(keys, 'b', 'ArrowUp', 3)).toBe('a');
    expect(moveSelection(keys, 'a', 'ArrowUp', 3)).toBe('a');
    expect(moveSelection(keys, 'e', 'ArrowDown', 3)).toBe('e');
    expect(moveSelection(keys, 'a', 'PageDown', 3)).toBe('d');
    expect(moveSelection(keys, 'd', 'PageDown', 3)).toBe('e');
    expect(moveSelection(keys, 'c', 'PageUp', 3)).toBe('a');
    expect(moveSelection(keys, 'c', 'Home', 3)).toBe('a');
    expect(moveSelection(keys, 'c', 'End', 3)).toBe('e');
  });

  it('starts from an end without a selection and ignores other keys', () => {
    expect(moveSelection(keys, null, 'ArrowDown', 3)).toBe('a');
    expect(moveSelection(keys, null, 'ArrowUp', 3)).toBe('e');
    expect(moveSelection(keys, 'gone', 'ArrowDown', 3)).toBe('a');
    for (const key of ['Enter', 'l', ' ', 'Escape', 'ArrowLeft']) expect(moveSelection(keys, 'b', key, 3), key).toBeUndefined();
    expect(moveSelection([], null, 'ArrowDown', 3)).toBeUndefined();
  });
});

describe('song list key ownership', () => {
  function list(): Element {
    document.body.innerHTML = `<ul role="listbox" tabindex="0" data-home data-owns-keys="${SONG_LIST_KEYS.join(' ')}"><li role="option"><span>Take</span></li></ul>`;
    return document.querySelector('ul')!;
  }
  it('owns its navigation keys and Enter, nothing else', () => {
    const target = list();
    for (const key of SONG_LIST_KEYS) expect(ownsKey(target, key, 'keyboard'), key).toBe(true);
    for (const key of SONG_LIST_KEYS) expect(ownsKey(document.querySelector('span'), key, 'pointer'), key).toBe(true);
  });
  it('lets Esc, Space, letters and ⌘ shortcuts reach the dispatcher', () => {
    const target = list();
    for (const key of ['Escape', ' ', 'n', 'i', 'f', ',', 'o', 'ArrowLeft', 'ArrowRight']) expect(ownsKey(target, key, 'keyboard'), key).toBe(false);
  });
});
