// Home focus: the visible screen's workspace (waveform, song list), marked
// with `data-home`. Committing a field, Esc and closing transient UI return here.
function homeElement(doc: Document): HTMLElement | null {
  return [...doc.querySelectorAll<HTMLElement>('[data-home]')].find(element => !element.closest('[hidden], [inert]')) ?? null;
}

export function focusHome(doc: Document = document): void {
  const home = homeElement(doc);
  if (home) home.focus({ preventScroll: true });
  else if (doc.activeElement instanceof HTMLElement) doc.activeElement.blur();
}

export function atHome(doc: Document = document): boolean {
  const active = doc.activeElement;
  return !active || active === doc.body || active === doc.documentElement || active === homeElement(doc);
}
