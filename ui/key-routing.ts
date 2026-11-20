// Decides which element owns a key press. Ownership depends on the key, never
// on the element's role alone: a composite control owns only its documented
// keys, and every other key reaches the global shortcut handler.
import type { Modality } from './input-modality';
import { NAVIGATION_KEYS } from './keymap';
const ACTIVATION_KEYS: readonly string[] = [' ', 'Enter'];
const NON_TEXT_INPUTS = new Set(['button', 'checkbox', 'color', 'file', 'hidden', 'image', 'radio', 'range', 'reset', 'submit']);

/** Text fields, textareas, selects and contenteditable regions own every key while focused. */
export function isTextEntry(element: Element): boolean {
  if (element instanceof HTMLInputElement) return !NON_TEXT_INPUTS.has(element.type);
  return element instanceof HTMLTextAreaElement || element instanceof HTMLSelectElement ||
    (element instanceof HTMLElement && element.isContentEditable) || element.closest('[contenteditable="true"]') !== null;
}

/** Single-line fields that commit on Enter and revert on Esc. */
export function isShortField(element: Element | null): element is HTMLInputElement {
  return element instanceof HTMLInputElement && isTextEntry(element);
}

/** Buttons and toggles: activated by Space/Enter, never focused by a click. */
export const ACTIVATABLE = 'button, [role="button"], [role="tab"], [role="switch"], [role="checkbox"], [role="radio"], [role="menuitem"], input[type="checkbox"], input[type="radio"], summary, a[href]';

/**
 * Keys a composite control documents. Custom controls (seek surfaces, knobs,
 * loop handles, map lines) declare them with `data-owns-keys="ArrowLeft ArrowRight"`;
 * a native range input owns the navigation keys it handles itself.
 */
function compositeKeys(element: Element): readonly string[] | null {
  const declared = element.closest<HTMLElement>('[data-owns-keys]');
  if (declared) return declared.dataset.ownsKeys!.split(/\s+/).filter(Boolean);
  if (element.closest('input[type="range"]')) return NAVIGATION_KEYS;
  return null;
}

/**
 * Keys a toolbar navigates between its buttons with (`data-nav-keys`). Like
 * Space and Enter on a button, they belong to it only during keyboard
 * navigation, and only while its focus arrived by keyboard: the toolbar marks
 * itself `data-keyboard-focus` when focus enters during keyboard navigation.
 * A button focused any other way (a restored focus, a click where WebView
 * focuses anyway) leaves the arrows to the shortcuts, even after the arrow
 * press itself switches the modality to keyboard.
 */
function navigationKeys(element: Element): readonly string[] {
  const toolbar = element.closest<HTMLElement>('[data-nav-keys]');
  return toolbar?.hasAttribute('data-keyboard-focus') ? toolbar.dataset.navKeys!.split(/\s+/).filter(Boolean) : [];
}

/** Marks a `data-nav-keys` toolbar while its focus arrived during keyboard navigation. */
export function trackKeyboardFocus(toolbar: HTMLElement, event: FocusEvent, modality: Modality): void {
  if (event.type === 'focusin') toolbar.toggleAttribute('data-keyboard-focus', modality === 'keyboard');
  else if (!toolbar.contains(event.relatedTarget as Node | null)) toolbar.removeAttribute('data-keyboard-focus');
}

/**
 * True when the focused target handles `key` itself. Open dialogs and
 * popovers (`data-popover`) own every key pressed inside them, so global shortcuts must
 * not run. Only keyboard-focused buttons and toggles own Space and Enter;
 * after a click (pointer modality) those keys reach the shortcuts.
 */
export function ownsKey(target: EventTarget | null, key: string, modality: Modality): boolean {
  if (!(target instanceof Element)) return false;
  if (target.closest('dialog[open], [data-popover]')) return true;
  if (isTextEntry(target)) return true;
  if (compositeKeys(target)?.includes(key)) return true;
  if (modality === 'keyboard' && navigationKeys(target).includes(key)) return true;
  return modality === 'keyboard' && ACTIVATION_KEYS.includes(key) && target.closest(ACTIVATABLE) !== null;
}
