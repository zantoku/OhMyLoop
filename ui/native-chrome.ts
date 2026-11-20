// App-wide pointer behaviour that makes the webview feel like a native app:
// clicks leave keyboard focus where it was, chrome has no browser context
// menu, and images, SVGs and links cannot be dragged out.
import { ACTIVATABLE, isTextEntry } from './key-routing';
import { focusHome } from './home-focus';

/** A press on a button or toggle must not move focus to it. */
export function keepsFocusOnPress(target: EventTarget | null): boolean {
  if (!(target instanceof Element) || isTextEntry(target)) return false;
  if (target.closest('[role="slider"], input[type="range"], [data-owns-keys]')) return false;
  const label = target.closest('label');
  if (label) return !(label.control && isTextEntry(label.control));
  return target.closest(ACTIVATABLE) !== null;
}

export function allowsContextMenu(target: EventTarget | null): boolean {
  return target instanceof Element && isTextEntry(target);
}

export function blocksDrag(target: EventTarget | null): boolean {
  return target instanceof Element && !target.closest('[draggable="true"]') && target.closest('img, svg, a') !== null;
}

export function installNativeChrome(doc: Document = document, home: () => void = () => focusHome(doc)): () => void {
  // WebKit and Chromium move focus as the default action of mousedown, so the
  // press-time event is cancelled here; pointerdown cannot prevent focus.
  const press = (event: MouseEvent) => {
    if (event.button !== 0 || !keepsFocusOnPress(event.target)) return;
    // Cancelling the press would also keep an edited field focused, so its
    // change never fires and the button acts on the stale value. Commit the
    // field first by sending focus home, which blurs it.
    const active = doc.activeElement;
    if (active instanceof Element && isTextEntry(active) && !active.contains(event.target as Node)) home();
    event.preventDefault();
  };
  const menu = (event: MouseEvent) => { if (!allowsContextMenu(event.target)) event.preventDefault(); };
  const drag = (event: DragEvent) => { if (blocksDrag(event.target)) event.preventDefault(); };
  doc.addEventListener('mousedown', press, true);
  doc.addEventListener('contextmenu', menu, true);
  doc.addEventListener('dragstart', drag, true);
  return () => {
    doc.removeEventListener('mousedown', press, true);
    doc.removeEventListener('contextmenu', menu, true);
    doc.removeEventListener('dragstart', drag, true);
  };
}
