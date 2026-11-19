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

it('reports where the shortcuts come from: built-in, the person’s file, or defaults after a rejection',async()=>{
  expect((await loadKeymap(async()=>({path:'/k.json',configuration:null,warning:null}),false)).source).toBe('builtin');
  expect((await loadKeymap(async()=>({path:'/k.json',configuration:{version:1,keys:{play_pause:['p']}},warning:null}),false)).source).toBe('custom');
  expect((await loadKeymap(async()=>({path:'/k.json',configuration:{version:2},warning:null}),false)).source).toBe('rejected');
  expect((await loadKeymap(async()=>({path:'/k.json',configuration:null,warning:'Cannot load /k.json: bad JSON'}),false)).source).toBe('rejected');
});
it('loads a file that still names the retired managed_settings, with a warning',async()=>{
  const loaded=await loadKeymap(async()=>({path:'/k.json',configuration:{version:1,keys:{managed_settings:['Alt+s']},buttons:{managed_settings:'managed_settings'}},warning:null}),false);
  expect(loaded.source).toBe('custom');expect(loaded.warning).toMatch(/Ignored keys\.managed_settings/);expect(loaded.warning).toMatch(/Ignored buttons\.managed_settings/);
});
it('allows the same key in different screens and matches only the active screen',()=>{
  const map=new Keymap(false,{version:1,keys:{map_beat:['a']}});
  const event=new KeyboardEvent('keydown',{key:'a'});
  expect(map.match(event,'practice')).toBe('set_loop_start');expect(map.match(event,'mapping')).toBe('map_beat');
  expect(map.shortcuts('mapping').some(row=>row.action==='set_loop_start')).toBe(false);
  expect(()=>new Keymap(false,{version:1,keys:{map_beat:['Space']}})).toThrow(/conflict/);
});

it('binds Esc, ⌘I and ⌘F on the Library only, and ⌘, to Settings everywhere',()=>{
  const keymap=new Keymap(true);
  expect(keymap.match(key('Escape'),'library')).toBe('library_close');
  for(const context of ['practice','mapping','stems'] as const)expect(keymap.match(key('Escape'),context)).toBe('disable_loop');
  expect(keymap.match(key('i',{metaKey:true}),'library')).toBe('library_edit');
  expect(keymap.match(key('f',{metaKey:true}),'library')).toBe('library_find');
  expect(keymap.match(key('i',{metaKey:true}),'practice')).toBeUndefined();
  expect(keymap.match(key('f',{metaKey:true}),'practice')).toBeUndefined();
  for(const context of ['practice','mapping','library','stems'] as const)expect(keymap.match(key(',',{metaKey:true}),context)).toBe('show_settings');
  expect(new Keymap(false).shortcut('show_settings')).toBe('Ctrl + ,');
  // Esc on the song screens and on the Library may not both claim a key for every screen.
  expect(()=>new Keymap(false,{version:1,keys:{show_shortcuts:['Escape']}})).toThrow(/conflict/);
});

it('lets an older configuration keep its keys when newer defaults would take them',()=>{
  const keymap=new Keymap(true,{version:1,keys:{show_shortcuts:['Mod+,'],show_library:['Mod+i'],reload_keymap:['Mod+f']}});
  expect(keymap.match(key(',',{metaKey:true}),'library')).toBe('show_shortcuts');
  expect(keymap.match(key('i',{metaKey:true}),'library')).toBe('show_library');
  expect(keymap.match(key('f',{metaKey:true}),'library')).toBe('reload_keymap');
  expect(keymap.shortcut('show_settings')).toBe('');expect(keymap.shortcut('library_edit')).toBe('');expect(keymap.shortcut('library_find')).toBe('');
  expect(keymap.warnings).toEqual([
    expect.stringMatching(/show_shortcuts replaces the default ⌘ \+ , of show_settings/),
    expect.stringMatching(/show_library replaces the default ⌘ \+ I of library_edit/),
    expect.stringMatching(/reload_keymap replaces the default ⌘ \+ F of library_find/)]);
  const esc=new Keymap(false,{version:1,keys:{library_find:['Escape']}});
  expect(esc.match(key('Escape'),'library')).toBe('library_find');expect(esc.match(key('Escape'),'practice')).toBe('disable_loop');
  expect(esc.warnings).toEqual([expect.stringMatching(/library_find replaces the default Esc of library_close/)]);
  // Two of the person's own bindings, or a binding against an older default, still conflict.
  expect(()=>new Keymap(false,{version:1,keys:{library_find:['Mod+j'],library_edit:['Mod+j']}})).toThrow(/conflict/);
  expect(()=>new Keymap(false,{version:1,keys:{library_find:['l']}})).not.toThrow();
  expect(()=>new Keymap(false,{version:1,keys:{show_shortcuts:['l']}})).toThrow(/conflict/);
});

it('ignores removed actions and buttons with a warning instead of rejecting the file',async()=>{
  const configuration={version:1,keys:{load_demo:['d'],play_pause:['p']},buttons:{demo:'load_demo',play:'play_pause'},descriptions:{load_demo:'Demo'}};
  const loaded=await loadKeymap(async()=>({path:'/k.json',configuration,warning:null}),false);
  expect(loaded.model.match(key('p'))).toBe('play_pause');expect(loaded.model.match(key('d'))).toBeUndefined();
  expect(loaded.warning).toMatch(/Ignored keys\.load_demo.*Ignored buttons\.demo.*Ignored descriptions\.load_demo/);
  expect(()=>new Keymap(false,{version:1,keys:{typo:['q']}})).toThrow(/Unknown/);
  expect(()=>new Keymap(false,{version:1,buttons:{play:'load_demo'}})).not.toThrow();
});

it('provides configurable library actions and validates contextual bindings',()=>{
  const keymap=new Keymap(false,{version:1,keys:{show_library:['Mod+k'],library_close:['q']}});
  expect(keymap.match(key('k',{ctrlKey:true}),'practice')).toBe('show_library');
  expect(keymap.match(key('q'),'library')).toBe('library_close');
  expect(keymap.match(key('q'),'mapping')).toBeUndefined();
  expect(keymap.match(key('Escape'),'mapping')).toBe('disable_loop');
  expect(()=>new Keymap(false,{version:1,keys:{library_sort:['s']}})).toThrow(/contextual/);
});

it('uses the registered batch grouping action for buttons, help and configured mapping shortcuts',()=>{
  expect(new Keymap().shortcut('map_grouping')).toBe('');
  const map=new Keymap(false,{version:1,keys:{map_grouping:['g']}});
  expect(map.actionFor('map_grouping')).toBe('map_grouping');
  expect(map.match(key('g'),'mapping')).toBe('map_grouping');
  expect(map.match(key('g'),'practice')).toBeUndefined();
  expect(map.shortcuts('mapping').find(row=>row.action==='map_grouping')?.description).toBe(map.description('map_grouping'));
  expect(map.title('map_grouping')).toBe(`${map.description('map_grouping')} (G)`);
});

it('toggles the notes drawer with N on every song view and saves a section with S',()=>{
  const map=new Keymap();
  for(const context of ['practice','mapping','stems'] as const)expect(map.match(key('n'),context)).toBe('show_notes');
  expect(map.match(key('N',{shiftKey:true}))).toBe('show_notes');
  expect(map.match(key('s'),'practice')).toBe('add_section');expect(map.match(key('s'),'mapping')).toBeUndefined();
  expect(map.title('show_notes')).toBe('Show or hide notes (N)');expect(map.title('save_selection')).toBe('Save a named section (S)');
  expect(map.keysFor('close_notes')).toEqual([]);expect(map.keysFor('retry_notes')).toEqual([]);
});
