import { describe, expect, it } from 'vitest';
import { Keymap, loadKeymap } from './keymap';
const key = (value: string, options: KeyboardEventInit = {}) => new KeyboardEvent('keydown', { key: value, ...options });

describe('keymap configuration model', () => {
  it('defines separate A/B jumps and Enter rewind, with one source for descriptions', () => {
    const map = new Keymap();
    expect(map.match(key(','))).toBe('jump_loop_start');
    expect(map.match(key('.'))).toBe('jump_loop_end');
    expect(map.match(key('Enter'))).toBe('jump_song_start');
    expect(map.title('jump_loop_start')).toBe('Jump to A (,)');
    expect(map.shortcuts().find(s => s.action === 'jump_loop_end')?.description).toBe(map.description('jump_loop_end'));
  });
  it('matches letter case and produced punctuation without confusing modified commands', () => {
    const map = new Keymap();
    expect(map.match(key('X', { shiftKey: true }))).toBe('clear_loop');
    expect(map.match(key('.', { shiftKey: true }))).toBe('jump_loop_end');
    expect(map.match(key('.', { ctrlKey: true }))).toBeUndefined();
    expect(map.match(key('.', { isComposing: true }))).toBeUndefined();
    expect(map.match(key('ArrowLeft', { shiftKey: true }))).toBe('seek_backward_fine');
  });
  it('uses Cmd on macOS and Ctrl elsewhere for Mod bindings', () => {
    expect(new Keymap(true).match(key('o', { metaKey: true }))).toBe('open_song');
    expect(new Keymap(true).match(key('o', { ctrlKey: true }))).toBeUndefined();
    expect(new Keymap().match(key('o', { ctrlKey: true }))).toBe('open_song');
    expect(new Keymap(true).shortcut('open_song')).toBe('⌘ + O');
  });
  it('distinguishes shifted command chords from their unshifted versions', () => {
    const map = new Keymap(false, { version: 1, keys: { open_song: ['Mod+p'], jump_song_start: ['Mod+Shift+p'] } });
    expect(map.match(key('p', { ctrlKey: true }))).toBe('open_song');
    expect(map.match(key('P', { ctrlKey: true, shiftKey: true }))).toBe('jump_song_start');
  });
  it('supports alternative keys and explicit disabling while inheriting omitted defaults', () => {
    const map = new Keymap(false, { version: 1, keys: { jump_loop_start: ['q', 'Home'], clear_loop: [] } });
    expect(map.match(key(','))).toBeUndefined(); expect(map.match(key('q'))).toBe('jump_loop_start');
    expect(map.match(key('Home'))).toBe('jump_loop_start'); expect(map.match(key('x'))).toBeUndefined();
    expect(map.match(key('Enter'))).toBe('jump_song_start');
  });
  it('rejects ambiguous keys, unsupported versions, unknown actions, and incompatible button payloads', () => {
    for (const configuration of [
      { version: 2 }, { version: 1, extra: true }, { version: 1, keys: { typo: ['q'] } },
      { version: 1, keys: { jump_loop_start: ['.'] } },
      { version: 1, keys: { jump_loop_start: ['r'], jump_loop_end: ['Shift+r'] } },
      { version: 1, keys: { jump_loop_start: ['Mod+Ctrl+r'] } },
      { version: 1, buttons: { play: 'delete_section' } },
      { version: 1, descriptions: { jump_loop_start: '' } }
    ]) expect(() => new Keymap(false, configuration)).toThrow();
  });
  it('ignores held toggles but allows repeated navigation', () => {
    const map = new Keymap();
    expect(map.match(key(' ', { repeat: true }))).toBeUndefined();
    expect(map.match(key('ArrowRight', { repeat: true }))).toBe('seek_forward');
  });
  it('falls back atomically on invalid configuration and preserves its source path', async () => {
    const loaded = await loadKeymap(async () => ({ path: '/test/keymap.json', configuration: { version: 1, keys: { play_pause: ['l'] } }, warning: null }), false);
    expect(loaded.warning).toContain('conflicts'); expect(loaded.path).toBe('/test/keymap.json');
    expect(loaded.model.match(key(' '))).toBe('play_pause');
  });
  it('uses defaults on missing files or I/O failures', async () => {
    const missing = await loadKeymap(async () => ({ path: '/missing/keymap.json', configuration: null, warning: null }), false);
    expect(missing.warning).toBeNull(); expect(missing.model.match(key(','))).toBe('jump_loop_start');
    const failure = await loadKeymap(async () => { throw new Error('read failed'); }, false);
    expect(failure.warning).toContain('read failed');
    expect(failure.model.match(key('Enter'))).toBe('jump_song_start');
  });
});

it('allows the same key in different screens and matches only the active screen',()=>{
  const map=new Keymap(false,{version:1,keys:{map_beat:['a']}});
  const event=new KeyboardEvent('keydown',{key:'a'});
  expect(map.match(event,'practice')).toBe('set_loop_start');expect(map.match(event,'mapping')).toBe('map_beat');
  expect(map.shortcuts('mapping').some(row=>row.action==='set_loop_start')).toBe(false);
  expect(()=>new Keymap(false,{version:1,keys:{map_beat:['Space']}})).toThrow(/conflict/);
});

it('provides configurable library actions with no default shortcut and validates contextual bindings',()=>{
  const keymap=new Keymap(false,{version:1,keys:{show_library:['Mod+k'],library_close:['q']}});
  expect(keymap.match(key('k',{ctrlKey:true}),'practice')).toBe('show_library');
  expect(keymap.match(key('q'),'library')).toBe('library_close');
  expect(keymap.match(key('q'),'mapping')).toBeUndefined();
  expect(keymap.match(key('Escape'),'mapping')).toBe('disable_loop');
  expect(()=>new Keymap(false,{version:1,keys:{library_sort:['s']}})).toThrow(/contextual/);
});
