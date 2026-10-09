import { ACTIONS, BUTTONS, type ActionId, type ButtonId } from './actions';

export type KeymapFile = { path: string | null; configuration: unknown; warning: string | null };
type Input = Pick<KeyboardEvent, 'key' | 'shiftKey' | 'ctrlKey' | 'metaKey' | 'altKey' | 'repeat' | 'isComposing'>;
type Chord = { key: string; shift: boolean | null; ctrl: boolean; meta: boolean; alt: boolean };
type Config = { version: 1; keys?: Partial<Record<ActionId, string[]>>; buttons?: Partial<Record<ButtonId, ActionId>>; descriptions?: Partial<Record<ActionId, string>> };
const own = (value: object, key: string) => Object.hasOwn(value, key);
const object = (value: unknown): value is Record<string, unknown> => value !== null && typeof value === 'object' && !Array.isArray(value);
const keyName = (key: string) => key === ' ' ? 'Space' : key === 'Plus' ? '+' : key.length === 1 ? key.toLowerCase() : key;
const namedKeys = new Set(['Space', 'Enter', 'Escape', 'ArrowLeft', 'ArrowRight', 'ArrowUp', 'ArrowDown', 'Home', 'End', 'PageUp', 'PageDown', 'Delete', 'Backspace']);

function parse(text: string, mac: boolean): Chord {
  const parts = text.split('+');
  const key = keyName(parts.pop() ?? '');
  if (!(key.length === 1 || namedKeys.has(key) || /^F(?:[1-9]|1[0-2])$/.test(key))) throw new Error(`Unsupported key: ${text}`);
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

  constructor(readonly mac = false, configuration?: unknown) {
    const config = configuration === undefined ? undefined : validate(configuration);
    this.keys = Object.fromEntries(Object.entries(ACTIONS).map(([id, definition]) => [id, Object.freeze([...(config?.keys?.[id as ActionId] ?? definition.keys)])])) as Record<ActionId, readonly string[]>;
    this.buttons = { ...BUTTONS, ...config?.buttons };
    this.descriptions = Object.fromEntries(Object.entries(ACTIONS).map(([id, definition]) => [id, config?.descriptions?.[id as ActionId]?.trim() ?? definition.description])) as Record<ActionId, string>;
    for (const id of Object.keys(ACTIONS) as ActionId[]) {
      for (const text of this.keys[id]) {
        const chord = parse(text, mac);
        const scope=ACTIONS[id].scope;
        const conflict = this.bindings.find(b => overlaps(b.chord, chord) && (scope==='all' || ACTIONS[b.action].scope==='all' || scope===ACTIONS[b.action].scope));
        if (conflict) throw new Error(`Shortcut ${text} conflicts with ${conflict.action}.`);
        this.bindings.push({ action: id, chord });
      }
    }
  }

  match(event: Input, context:'practice'|'mapping'='practice'): ActionId | undefined {
    if (event.isComposing) return;
    return this.bindings.find(({ action, chord }) => (ACTIONS[action].scope==='all'||ACTIONS[action].scope===context) && chord.key === keyName(event.key) &&
      chord.ctrl === event.ctrlKey && chord.meta === event.metaKey && chord.alt === event.altKey &&
      (chord.shift === null || chord.shift === event.shiftKey) && (!event.repeat || ACTIONS[action].repeat))?.action;
  }

  actionFor(button: ButtonId): ActionId { return this.buttons[button]; }
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
  shortcuts(context?:'practice'|'mapping'): { action: ActionId; keys: string; description: string }[] {
    return (Object.keys(ACTIONS) as ActionId[]).filter(id => this.keys[id].length && (!context || ACTIONS[id].scope==='all' || ACTIONS[id].scope===context)).map(action => ({ action, keys: this.shortcut(action), description: this.description(action) }));
  }
  private display(text: string): string {
    return text.split('+').map(part => ({ Mod: this.mac ? '⌘' : 'Ctrl', Meta: this.mac ? '⌘' : 'Meta', ArrowLeft: '←', ArrowRight: '→', ArrowUp: '↑', ArrowDown: '↓', Escape: 'Esc' }[part] ?? (part.length === 1 && /[a-z]/i.test(part) ? part.toUpperCase() : part))).join(' + ');
  }
}

export async function loadKeymap(read: () => Promise<KeymapFile>, mac: boolean): Promise<{ model: Keymap; path: string | null; warning: string | null }> {
  let path: string | null = null;
  try {
    const source = await read(); path = source.path;
    if (source.warning) return { model: new Keymap(mac), path, warning: source.warning };
    return { model: new Keymap(mac, source.configuration ?? undefined), path, warning: null };
  } catch (error) {
    return { model: new Keymap(mac), path, warning: `Using default shortcuts: ${String(error)}` };
  }
}
