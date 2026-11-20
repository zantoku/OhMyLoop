// Keyboard selection in the Library's song list, a single-select listbox.
// The list owns exactly these keys; every other key reaches the shortcuts.
export const SONG_LIST_KEYS = ['ArrowUp', 'ArrowDown', 'Home', 'End', 'PageUp', 'PageDown', 'Enter'] as const;

/**
 * The song selected after a navigation key, or undefined when the key doesn't
 * move the selection. Without a selection, the first move picks an end.
 */
export function moveSelection(keys: readonly string[], current: string | null, key: string, page: number): string | undefined {
  if (!keys.length) return undefined;
  const index = current === null ? -1 : keys.indexOf(current);
  const last = keys.length - 1;
  const step = (by: number) => index < 0 ? (by > 0 ? 0 : last) : Math.min(last, Math.max(0, index + by));
  const target = { ArrowDown: step(1), ArrowUp: step(-1), PageDown: step(Math.max(1, page)), PageUp: step(-Math.max(1, page)), Home: 0, End: last }[key];
  return target === undefined ? undefined : keys[target];
}
