// Open transient UI (popovers, menus), newest last. Esc unwinds one level per
// press, so the global key handler closes the newest one before anything else.
const open: (() => void)[] = [];

/** Registers an open popover; call the returned function when it closes. */
export function openTransient(close: () => void): () => void {
  open.push(close);
  return () => { const index = open.lastIndexOf(close); if (index >= 0) open.splice(index, 1); };
}

/** Closes the newest open transient UI; false when none is open. */
export function closeTopTransient(): boolean {
  const close = open.pop();
  close?.();
  return !!close;
}
