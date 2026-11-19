import { ACTIONS, BUTTONS, inScope, scopesOverlap, type ActionContext, type ActionId, type ButtonId } from './actions';

export type KeymapFile = { path: string | null; configuration: unknown; warning: string | null };
/** Where the shortcuts in use come from: no file, the person's file, or the defaults because their file was rejected. */
export type KeymapSource = 'builtin' | 'custom' | 'rejected';
type Input = Pick<KeyboardEvent, 'key' | 'shiftKey' | 'ctrlKey' | 'metaKey' | 'altKey' | 'repeat' | 'isComposing'>;
export type Chord = { key: string; shift: boolean | null; ctrl: boolean; meta: boolean; alt: boolean };
type Config = { version: 1; keys?: Partial<Record<ActionId, string[]>>; buttons?: Partial<Record<ButtonId, ActionId>>; descriptions?: Partial<Record<ActionId, string>> };
const own = (value: object, key: string) => Object.hasOwn(value, key);
const object = (value: unknown): value is Record<string, unknown> => value !== null && typeof value === 'object' && !Array.isArray(value);
const keyName = (key: string) => key === ' ' ? 'Space' : key === 'Plus' ? '+' : key.length === 1 ? key.toLowerCase() : key;
/** Keys a slider-like control navigates with; also the keys that signal keyboard navigation. */
export const NAVIGATION_KEYS: readonly string[] = ['ArrowLeft', 'ArrowRight', 'ArrowUp', 'ArrowDown', 'Home', 'End', 'PageUp', 'PageDown'];
/** Named (non-character) keys a binding may use, besides F1–F12. */
export const NAMED_KEYS: ReadonlySet<string> = new Set(['Space', 'Enter', 'Escape', ...NAVIGATION_KEYS, 'Delete', 'Backspace']);

/** Parses a binding such as `Mod+Shift+z`; `Plus` means a literal `+`. Throws on invalid text. */
export function parseChord(text: string, mac: boolean): Chord {
  const parts = text.split('+');
  const key = keyName(parts.pop() ?? '');
  if (!(key.length === 1 || NAMED_KEYS.has(key) || /^F(?:[1-9]|1[0-2])$/.test(key))) throw new Error(`Unsupported key: ${text}`);
  const modifiers = new Set(parts);
  if (modifiers.size !== parts.length || parts.some(p => !['Mod', 'Ctrl', 'Meta', 'Alt', 'Shift'].includes(p))) throw new Error(`Invalid modifiers: ${text}`);
  if (modifiers.has('Mod') && (modifiers.has('Meta') || modifiers.has('Ctrl'))) throw new Error(`Do not combine Mod with Ctrl or Meta: ${text}`);
  const ctrl = modifiers.has('Ctrl') || (modifiers.has('Mod') && !mac);
  const meta = modifiers.has('Meta') || (modifiers.has('Mod') && mac);
  const alt = modifiers.has('Alt');
  return { key, shift: modifiers.has('Shift') ? true : key.length === 1 && !ctrl && !meta && !alt ? null : false, ctrl, meta, alt };
}

function overlaps(a: Chord, b: Chord): boolean {
  return a.key === b.key && a.ctrl === b.ctrl && a.meta === b.meta && a.alt === b.alt &&
    (a.shift === null || b.shift === null || a.shift === b.shift);
}

/**
 * Identities that existed in older versions. A configuration that still names
 * them loads with a warning instead of being rejected.
 */
const REMOVED_ACTIONS: ReadonlySet<string> = new Set(['load_demo', 'managed_settings']);
const REMOVED_BUTTONS: ReadonlySet<string> = new Set(['demo', 'managed_settings']);
/**
 * Defaults added after configurations could already use their keys. One of
 * these gives way to a person's own binding of the same key, with a warning;
 * any other collision is an error.
 */
const NEWER_DEFAULTS: ReadonlySet<ActionId> = new Set(['library_close', 'library_edit', 'library_find', 'show_settings']);

/** Drops removed identities from a configuration, describing each one dropped. */
function withoutRemoved(value: unknown, warnings: string[]): unknown {
  if (!object(value)) return value;
  const copy: Record<string, unknown> = { ...value };
  for (const [name, removed] of [['keys', REMOVED_ACTIONS], ['buttons', REMOVED_BUTTONS], ['descriptions', REMOVED_ACTIONS]] as const) {
    const section = copy[name];
    if (!object(section)) continue;
    const kept = Object.fromEntries(Object.entries(section).filter(([id, action]) => {
      const gone = removed.has(id) || (name === 'buttons' && typeof action === 'string' && REMOVED_ACTIONS.has(action));
      if (gone) warnings.push(`Ignored ${name}.${id}: it no longer exists in OhMyLoop.`);
      return !gone;
    }));
    copy[name] = kept;
  }
  return copy;
}

function validate(value: unknown): Config {
  if (!object(value) || value.version !== 1) throw new Error('Shortcut configuration must have version 1.');
  if (Object.keys(value).some(k => !['version', 'keys', 'buttons', 'descriptions'].includes(k))) throw new Error('Unknown shortcut configuration field.');
  for (const name of ['keys', 'buttons', 'descriptions']) {
    if (value[name] !== undefined && !object(value[name])) throw new Error(`${name} must be an object.`);
  }
  for (const [id, keys] of Object.entries(value.keys ?? {})) {
    if (!own(ACTIONS, id) || ACTIONS[id as ActionId].payload !== 'command') throw new Error(`Unknown or contextual keyboard action: ${id}`);
    if (!Array.isArray(keys) || keys.length > 8 || keys.some(k => typeof k !== 'string' || !k || k.length > 80)) throw new Error(`Invalid keys for ${id}.`);
  }
  for (const [button, action] of Object.entries(value.buttons ?? {})) {
    if (!own(BUTTONS, button) || typeof action !== 'string' || !own(ACTIONS, action)) throw new Error(`Unknown button/action mapping: ${button}`);
    const kind = ACTIONS[BUTTONS[button as ButtonId]].payload;
    if (ACTIONS[action as ActionId].payload !== kind) throw new Error(`Incompatible action for button ${button}.`);
  }
  for (const [id, description] of Object.entries(value.descriptions ?? {})) {
    if (!own(ACTIONS, id) || typeof description !== 'string' || !description.trim() || description.length > 160) throw new Error(`Invalid description for ${id}.`);
  }
  return value as Config;
}

export class Keymap {
  private bindings: { action: ActionId; chord: Chord }[] = [];
  private keys: Record<ActionId, readonly string[]>;
  private buttons: Record<ButtonId, ActionId>;
  private descriptions: Record<ActionId, string>;
  /** Problems the configuration had that didn't stop it from loading. */
  readonly warnings: string[] = [];

  constructor(readonly mac = false, configuration?: unknown) {
    const config = configuration === undefined ? undefined : validate(withoutRemoved(configuration, this.warnings));
    this.keys = Object.fromEntries(Object.entries(ACTIONS).map(([id, definition]) => [id, Object.freeze([...(config?.keys?.[id as ActionId] ?? definition.keys)])])) as Record<ActionId, readonly string[]>;
    this.buttons = { ...BUTTONS, ...config?.buttons };
    this.descriptions = Object.fromEntries(Object.entries(ACTIONS).map(([id, definition]) => [id, config?.descriptions?.[id as ActionId]?.trim() ?? definition.description])) as Record<ActionId, string>;
    // The person's own bindings come first. Two of them that collide are an
    // error; a newer default that collides with one of them gives way.
    const configured = (id: ActionId) => !!config?.keys && Object.hasOwn(config.keys, id);
    const ids = Object.keys(ACTIONS) as ActionId[];
    for (const id of [...ids.filter(configured), ...ids.filter(id => !configured(id))]) {
      const kept: string[] = [];
      for (const text of this.keys[id]) {
        const chord = parseChord(text, mac);
        const conflict = this.bindings.find(b => overlaps(b.chord, chord) && scopesOverlap(ACTIONS[id].scope, ACTIONS[b.action].scope));
        if (conflict && !configured(id) && NEWER_DEFAULTS.has(id) && configured(conflict.action)) {
          this.warnings.push(`Your shortcut for ${conflict.action} replaces the default ${this.display(text)} of ${id}.`);
          continue;
        }
        if (conflict) throw new Error(`Shortcut ${text} conflicts with ${conflict.action}.`);
        this.bindings.push({ action: id, chord });
        kept.push(text);
      }
      if (kept.length !== this.keys[id].length) this.keys[id] = Object.freeze(kept);
    }
  }

  match(event: Input, context: ActionContext = 'practice'): ActionId | undefined {
    if (event.isComposing) return;
    return this.bindings.find(({ action, chord }) => inScope(ACTIONS[action].scope, context) && chord.key === keyName(event.key) &&
      chord.ctrl === event.ctrlKey && chord.meta === event.metaKey && chord.alt === event.altKey &&
      (chord.shift === null || chord.shift === event.shiftKey) && (!event.repeat || ACTIONS[action].repeat))?.action;
  }

  actionFor(button: ButtonId): ActionId { return this.buttons[button]; }
  /** Resolved binding texts for an action, e.g. `Mod+o`; the menu bar derives accelerators from them. */
  keysFor(action: ActionId): readonly string[] { return this.keys[action]; }
  description(action: ActionId): string { return this.descriptions[action]; }
  shortcut(action: ActionId): string { return this.keys[action].map(k => this.display(k)).join(' / '); }
  actionTitle(action: ActionId): string {
    const keys = this.shortcut(action);
    return `${this.description(action)}${keys ? ` (${keys})` : ''}`;
  }
  title(button: ButtonId): string { return this.actionTitle(this.actionFor(button)); }
  buttonLabel(button: ButtonId, defaultLabel: string): string {
    return this.actionFor(button) === BUTTONS[button] ? defaultLabel : this.description(this.actionFor(button));
  }
  shortcuts(context?: ActionContext): { action: ActionId; keys: string; description: string }[] {
    return (Object.keys(ACTIONS) as ActionId[]).filter(id => this.keys[id].length && (!context || inScope(ACTIONS[id].scope, context))).map(action => ({ action, keys: this.shortcut(action), description: this.description(action) }));
  }
  private display(text: string): string {
    return text.split('+').map(part => ({ Mod: this.mac ? '⌘' : 'Ctrl', Meta: this.mac ? '⌘' : 'Meta', ArrowLeft: '←', ArrowRight: '→', ArrowUp: '↑', ArrowDown: '↓', Escape: 'Esc' }[part] ?? (part.length === 1 && /[a-z]/i.test(part) ? part.toUpperCase() : part))).join(' + ');
  }
}

export async function loadKeymap(read: () => Promise<KeymapFile>, mac: boolean): Promise<{ model: Keymap; path: string | null; warning: string | null; source: KeymapSource }> {
  let path: string | null = null;
  try {
    const source = await read(); path = source.path;
    if (source.warning) return { model: new Keymap(mac), path, warning: source.warning, source: 'rejected' };
    if (source.configuration === null || source.configuration === undefined) return { model: new Keymap(mac), path, warning: null, source: 'builtin' };
    const model = new Keymap(mac, source.configuration);
    return { model, path, warning: model.warnings.length ? model.warnings.join(' ') : null, source: 'custom' };
  } catch (error) {
    return { model: new Keymap(mac), path, warning: `Using default shortcuts: ${String(error)}`, source: 'rejected' };
  }
}
