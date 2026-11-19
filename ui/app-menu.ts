// The macOS menu bar, built from the action model so labels, accelerators and
// enabled state match the buttons and the shortcut list. Windows and Linux
// have no menu bar; their Ctrl shortcuts go through the webview handler.
import type { PredefinedMenuItemOptions } from '@tauri-apps/api/menu';
import { ACTION_LABELS, ACTIONS, type ActionId } from './actions';
import { NAMED_KEYS, parseChord, type Keymap } from './keymap';
import type { EditCommand } from './undo-routing';

type NativeItem = Extract<PredefinedMenuItemOptions['item'], string> | 'About';
export type MenuEntry =
  | { action: ActionId; text: string; accelerator: string | null }
  | { edit: EditCommand; text: string; accelerator: string }
  | { native: NativeItem };
export type MenuSection = { title: string; entries: MenuEntry[] };

const LAYOUT: { title: string; entries: (ActionId | EditCommand | NativeItem)[] }[] = [
  { title: 'OhMyLoop', entries: ['About', 'Separator', 'show_settings', 'Separator', 'Services', 'Separator', 'Hide', 'HideOthers', 'ShowAll', 'Separator', 'Quit'] },
  { title: 'File', entries: ['open_song', 'show_library', 'Separator', 'library_edit', 'replace_recording'] },
  // Undo/Redo are custom so they reach the app's undo outside text fields.
  { title: 'Edit', entries: ['undo', 'redo', 'Separator', 'Cut', 'Copy', 'Paste', 'SelectAll', 'Separator', 'library_find'] },
  { title: 'View', entries: ['show_practice', 'show_measure_map', 'show_notes', 'show_stems', 'Separator', 'toggle_sections', 'Separator', 'zoom_in', 'zoom_out', 'show_whole_song', 'Separator', 'Fullscreen'] },
  { title: 'Playback', entries: ['play_pause', 'jump_song_start', 'Separator', 'practice_count_in', 'Separator', 'toggle_loop', 'repeat_count_in', 'set_loop_start', 'set_loop_end', 'jump_loop_start', 'jump_loop_end', 'clear_loop', 'Separator', 'add_section'] },
  { title: 'Window', entries: ['Minimize', 'Maximize', 'Separator', 'BringAllToFront'] },
  { title: 'Help', entries: ['show_shortcuts', 'reload_keymap'] }
];
const EDIT: Record<EditCommand, { text: string; accelerator: string }> = {
  undo: { text: 'Undo', accelerator: 'Cmd+Z' },
  redo: { text: 'Redo', accelerator: 'Cmd+Shift+Z' }
};
// Characters the native accelerator parser accepts (US key positions).
const MENU_CHARACTERS = /^[a-z0-9`\\[\],=\-.';/]$/;

/**
 * A macOS menu accelerator for a keymap binding, or null. Only chords with
 * Command, Control or Option belong in the menu; single keys stay in the webview.
 */
export function accelerator(binding: string): string | null {
  let chord;
  try { chord = parseChord(binding, true); } catch { return null; }
  if (!chord.meta && !chord.ctrl && !chord.alt) return null;
  const key = MENU_CHARACTERS.test(chord.key) ? chord.key.toUpperCase() : NAMED_KEYS.has(chord.key) || /^F\d+$/.test(chord.key) ? chord.key : null;
  if (!key) return null;
  return [chord.meta && 'Cmd', chord.ctrl && 'Ctrl', chord.alt && 'Alt', chord.shift && 'Shift', key].filter(Boolean).join('+');
}

export function menuLayout(keymap: Keymap): MenuSection[] {
  return LAYOUT.map(({ title, entries }) => ({
    title,
    entries: entries.map((entry): MenuEntry => {
      if (entry === 'undo' || entry === 'redo') return { edit: entry, ...EDIT[entry] };
      if (!Object.hasOwn(ACTIONS, entry)) return { native: entry as NativeItem };
      const action = entry as ActionId;
      return { action, text: ACTION_LABELS[action] ?? keymap.description(action), accelerator: keymap.keysFor(action).map(accelerator).find(Boolean) ?? null };
    })
  }));
}

type Closable = { close(): Promise<void> };
type Item = Closable & { setEnabled(enabled: boolean): Promise<void> };
export type MenuCommands = { run: (id: ActionId) => void; edit: (command: EditCommand) => void };

/**
 * A built menu bar. `build` creates it, `activate` installs it, and `close`
 * releases its native resources when a rebuilt menu replaces it.
 */
export class AppMenu {
  private enabled = new Map<ActionId, boolean>();
  private constructor(private menu: { setAsAppMenu(): Promise<unknown> }, private items: Map<ActionId, Item>, private resources: Closable[]) {}

  get actions(): ActionId[] { return [...this.items.keys()]; }

  static async build(keymap: Keymap, commands: MenuCommands): Promise<AppMenu> {
    const { Menu, Submenu, MenuItem, PredefinedMenuItem } = await import('@tauri-apps/api/menu');
    const items = new Map<ActionId, Item>(), resources: Closable[] = [];
    const keep = <T extends Closable>(resource: T): T => { resources.push(resource); return resource; };
    const submenus = await Promise.all(menuLayout(keymap).map(async section => keep(await Submenu.new({
      text: section.title,
      items: await Promise.all(section.entries.map(async entry => {
        if ('native' in entry) return keep(await PredefinedMenuItem.new({ item: entry.native === 'About' ? { About: null } : entry.native }));
        if ('edit' in entry) return keep(await MenuItem.new({ id: `edit_${entry.edit}`, text: entry.text, accelerator: entry.accelerator, action: () => commands.edit(entry.edit) }));
        const item = keep(await MenuItem.new({ id: entry.action, text: entry.text, accelerator: entry.accelerator ?? undefined, enabled: false, action: () => commands.run(entry.action) }));
        items.set(entry.action, item);
        return item;
      }))
    }))));
    const menu = keep(await Menu.new({ items: submenus }));
    return new AppMenu(menu, items, resources);
  }

  async activate(): Promise<void> { await this.menu.setAsAppMenu(); }

  async close(): Promise<void> { await Promise.allSettled(this.resources.map(resource => resource.close())); }

  /** Applies the enabled actions, calling into the native menu only for changes. */
  sync(enabled: ReadonlySet<ActionId>): void {
    for (const [id, item] of this.items) {
      const on = enabled.has(id);
      if (this.enabled.get(id) === on) continue;
      this.enabled.set(id, on);
      void item.setEnabled(on).catch(() => this.enabled.delete(id));
    }
  }
}
