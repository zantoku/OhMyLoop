import { describe, expect, it, vi } from 'vitest';
import { accelerator, AppMenu, menuLayout, type MenuEntry } from './app-menu';
import { Keymap } from './keymap';

const tauri = vi.hoisted(() => {
  type Created = { id?: string; item?: unknown; action?: () => void; setEnabled: ReturnType<typeof vi.fn>; close: ReturnType<typeof vi.fn> };
  const created: Created[] = [];
  const resource = (options: object) => { const item = { ...options, setEnabled: vi.fn(async () => {}), close: vi.fn(async () => {}) }; created.push(item); return item; };
  const setAsAppMenu = vi.fn(async () => {});
  return {
    created, setAsAppMenu,
    module: {
      Menu: { new: vi.fn(async (options: object) => ({ ...resource(options), setAsAppMenu })) },
      Submenu: { new: vi.fn(async (options: object) => resource(options)) },
      PredefinedMenuItem: { new: vi.fn(async (options: object) => resource(options)) },
      MenuItem: { new: vi.fn(async (options: object) => resource(options)) }
    }
  };
});
vi.mock('@tauri-apps/api/menu', () => tauri.module);

const find = (keymap: Keymap, title: string) => menuLayout(keymap).find(section => section.title === title)!.entries;
const action = (entries: MenuEntry[], id: string) => entries.find(entry => 'action' in entry && entry.action === id) as Extract<MenuEntry, { action: string }>;
const natives = (entries: MenuEntry[]) => entries.flatMap(entry => 'native' in entry && entry.native !== 'Separator' ? [entry.native] : []);

describe('menu accelerators', () => {
  it('converts modifier chords with the keymap parser and leaves single keys to the webview', () => {
    expect(accelerator('Mod+o')).toBe('Cmd+O');
    expect(accelerator('Mod+Shift+z')).toBe('Cmd+Shift+Z');
    expect(accelerator('Ctrl+Alt+ArrowLeft')).toBe('Ctrl+Alt+ArrowLeft');
    expect(accelerator('Mod+,')).toBe('Cmd+,');
    expect(accelerator('Mod+F5')).toBe('Cmd+F5');
    for (const single of ['Space', 'l', 'Shift+ArrowLeft', 'Enter', 'Escape']) expect(accelerator(single), single).toBeNull();
    // `Plus` is the keymap's literal "+", which has no physical menu key.
    expect(accelerator('Mod+Plus')).toBeNull();
    expect(accelerator('Mod+é')).toBeNull();
    expect(accelerator('Bogus+x')).toBeNull();
  });
});

describe('menu layout from the action model', () => {
  it('keeps the predefined app items and Edit clipboard items, with custom Undo/Redo', () => {
    const keymap = new Keymap(true);
    expect(natives(find(keymap, 'OhMyLoop'))).toEqual(['About', 'Services', 'Hide', 'HideOthers', 'ShowAll', 'Quit']);
    const edit = find(keymap, 'Edit');
    expect(natives(edit)).toEqual(['Cut', 'Copy', 'Paste', 'SelectAll']);
    expect(edit.slice(0, 2)).toEqual([{ edit: 'undo', text: 'Undo', accelerator: 'Cmd+Z' }, { edit: 'redo', text: 'Redo', accelerator: 'Cmd+Shift+Z' }]);
  });

  it('takes labels and accelerators from the keymap, including custom bindings', () => {
    expect(action(find(new Keymap(true), 'File'), 'open_song')).toEqual({ action: 'open_song', text: 'Import Song…', accelerator: 'Cmd+O' });
    expect(action(find(new Keymap(true), 'Playback'), 'play_pause').accelerator).toBeNull();
    const custom = new Keymap(true, { version: 1, keys: { open_song: ['Mod+i'], show_library: ['Mod+l'] } });
    expect(action(find(custom, 'File'), 'open_song').accelerator).toBe('Cmd+I');
    expect(action(find(custom, 'File'), 'show_library').accelerator).toBe('Cmd+L');
    expect(action(find(new Keymap(true), 'Help'), 'show_shortcuts').text).toBe('Keyboard Shortcuts');
  });
});

describe('built menu', () => {
  it('routes commands, syncs enabled state on change only, and releases its resources', async () => {
    tauri.created.length = 0;
    const run = vi.fn(), edit = vi.fn();
    const menu = await AppMenu.build(new Keymap(true), { run, edit });
    expect(tauri.setAsAppMenu).not.toHaveBeenCalled();
    await menu.activate();
    expect(tauri.setAsAppMenu).toHaveBeenCalledOnce();
    expect(menu.actions).toContain('open_song');

    const open = tauri.created.find(item => item.id === 'open_song')!;
    open.action!();
    expect(run).toHaveBeenCalledWith('open_song');
    tauri.created.find(item => item.id === 'edit_undo')!.action!();
    tauri.created.find(item => item.id === 'edit_redo')!.action!();
    expect(edit.mock.calls).toEqual([['undo'], ['redo']]);

    menu.sync(new Set(['open_song']));
    expect(open.setEnabled).toHaveBeenCalledWith(true);
    menu.sync(new Set(['open_song']));
    expect(open.setEnabled).toHaveBeenCalledTimes(1);
    menu.sync(new Set());
    expect(open.setEnabled).toHaveBeenLastCalledWith(false);

    await menu.close();
    expect(tauri.created.every(resource => resource.close.mock.calls.length === 1)).toBe(true);
  });
});
