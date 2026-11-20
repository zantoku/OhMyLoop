import { describe, expect, it } from 'vitest';
import { ownsKey, trackKeyboardFocus } from './key-routing';
import { NAVIGATION_KEYS } from './keymap';

function element(html: string, selector: string): Element {
  const host = document.createElement('div');
  host.innerHTML = html;
  document.body.replaceChildren(host);
  return host.querySelector(selector)!;
}
const LETTERS_AND_TRANSPORT = ['l', 'a', 'x', ' ', 'Enter', 'Escape', ',', '['];

describe('key ownership is decided by the key', () => {
  it('lets text entry own every key', () => {
    for (const html of ['<input>', '<input type="number">', '<input type="search">', '<textarea></textarea>', '<select></select>', '<div contenteditable="true"><span></span></div>']) {
      const target = element(html, 'input, textarea, select, span');
      for (const key of [...LETTERS_AND_TRANSPORT, 'ArrowLeft']) expect(ownsKey(target, key, 'pointer'), `${html} ${key}`).toBe(true);
    }
  });

  it('lets a range input own only navigation keys; letters, Space and Esc reach the shortcuts', () => {
    const slider = element('<input type="range">', 'input');
    for (const key of NAVIGATION_KEYS) expect(ownsKey(slider, key, 'keyboard')).toBe(true);
    for (const key of LETTERS_AND_TRANSPORT) expect(ownsKey(slider, key, 'keyboard'), key).toBe(false);
  });

  it('gives a custom slider only the keys it declares, never all keys by role', () => {
    const seek = element('<div role="slider" tabindex="0" data-owns-keys="ArrowLeft ArrowRight Home End"></div>', 'div');
    for (const key of ['ArrowLeft', 'ArrowRight', 'Home', 'End']) expect(ownsKey(seek, key, 'keyboard'), key).toBe(true);
    for (const key of ['ArrowUp', 'PageDown', ...LETTERS_AND_TRANSPORT]) expect(ownsKey(seek, key, 'keyboard'), key).toBe(false);
    const undeclared = element('<div role="slider" tabindex="0"></div>', 'div');
    for (const key of [...NAVIGATION_KEYS, ...LETTERS_AND_TRANSPORT]) expect(ownsKey(undeclared, key, 'keyboard'), key).toBe(false);
  });

  it('lets a declared composite own exactly its documented keys', () => {
    const handle = element('<button data-owns-keys="ArrowLeft ArrowRight"><span>A</span></button>', 'span');
    expect(ownsKey(handle, 'ArrowLeft', 'pointer')).toBe(true);
    expect(ownsKey(handle, 'ArrowRight', 'pointer')).toBe(true);
    for (const key of ['ArrowUp', 'Home', 'l', ' ']) expect(ownsKey(handle, key, 'pointer'), key).toBe(false);
  });

  it('lets a toolbar own its navigation keys only during keyboard navigation (Measure map toolbar)', () => {
    const tool = element('<div role="toolbar" data-nav-keys="ArrowLeft ArrowRight Home End" data-keyboard-focus><button data-tool><svg></svg></button></div>', 'svg');
    for (const key of ['ArrowLeft', 'ArrowRight', 'Home', 'End']) {
      expect(ownsKey(tool, key, 'keyboard'), key).toBe(true);
      // After a click the arrows seek again, as everywhere else.
      expect(ownsKey(tool, key, 'pointer'), key).toBe(false);
    }
    for (const key of ['ArrowUp', 'PageDown', 'l', 's', 'm', 'Escape', ',']) expect(ownsKey(tool, key, 'keyboard'), key).toBe(false);
    // The focused button still owns Space and Enter while navigating by keyboard, and not after a click.
    expect(ownsKey(tool, ' ', 'keyboard')).toBe(true);expect(ownsKey(tool, ' ', 'pointer')).toBe(false);
  });

  it('leaves the arrows to the shortcuts when a toolbar button was focused other than by keyboard', () => {
    // On Windows and Linux the arrow press itself switches the modality to
    // keyboard; a button that got focus from a click must not take it.
    const tool = element('<div role="toolbar" data-nav-keys="ArrowLeft ArrowRight Home End"><button data-tool>Split</button></div>', 'button');
    const toolbar = tool.parentElement!;
    trackKeyboardFocus(toolbar, new FocusEvent('focusin'), 'pointer');
    for (const key of ['ArrowLeft', 'ArrowRight', 'Home', 'End']) expect(ownsKey(tool, key, 'keyboard'), key).toBe(false);
    trackKeyboardFocus(toolbar, new FocusEvent('focusin'), 'keyboard');
    expect(ownsKey(tool, 'ArrowRight', 'keyboard')).toBe(true);
    trackKeyboardFocus(toolbar, new FocusEvent('focusout', { relatedTarget: document.body }), 'keyboard');
    expect(ownsKey(tool, 'ArrowRight', 'keyboard')).toBe(false);
  });

  it('lets map beat diamonds own their nudge and Delete keys, and nothing else', () => {
    const diamond = element('<button data-owns-keys="ArrowLeft ArrowRight Delete Backspace" aria-label="Measure 1 beat 2"></button>', 'button');
    for (const key of ['ArrowLeft', 'ArrowRight', 'Delete', 'Backspace']) expect(ownsKey(diamond, key, 'pointer'), key).toBe(true);
    for (const key of ['ArrowUp', 'Home', 'd', 's', 'j', 'l', 'm', 'f', 'w', '[', ']', 'Escape']) expect(ownsKey(diamond, key, 'keyboard'), key).toBe(false);
    expect(ownsKey(diamond, ' ', 'pointer')).toBe(false);
  });

  it('lets panels and the drafting drawer fields own their keys while focused', () => {
    const panel = element('<div role="dialog" data-popover aria-label="m. 37 properties"><button type="button">Pickup</button></div>', 'button');
    for (const key of ['s', 'j', 'l', ' ', 'ArrowLeft', 'm']) expect(ownsKey(panel, key, 'pointer'), key).toBe(true);
    const drawer = element('<aside aria-label="Draft a measure map"><input aria-label="Number of measures" type="number"><button>Preview draft</button></aside>', 'input');
    for (const key of [' ', 's', 'Enter', 'ArrowLeft']) expect(ownsKey(drawer, key, 'pointer'), key).toBe(true);
    // A drawer button is a plain button: letters, and Space after a click, reach the shortcuts.
    const preview = document.querySelector('aside button')!;
    for (const key of ['s', 'm', ' ']) expect(ownsKey(preview, key, 'pointer'), key).toBe(false);
  });

  it('lets only keyboard-focused buttons and toggles own Space and Enter', () => {
    for (const html of ['<button><svg></svg></button>', '<input type="checkbox">', '<div role="button" tabindex="0"></div>', '<details><summary>More</summary></details>']) {
      const control = element(html, 'svg, input, [role=button], summary');
      expect(ownsKey(control, ' ', 'keyboard'), html).toBe(true);
      expect(ownsKey(control, 'Enter', 'keyboard'), html).toBe(true);
      // After a click (pointer modality) Space plays and Enter rewinds.
      expect(ownsKey(control, ' ', 'pointer'), html).toBe(false);
      expect(ownsKey(control, 'Enter', 'pointer'), html).toBe(false);
      for (const key of ['l', 'ArrowLeft', 'Escape']) expect(ownsKey(control, key, 'keyboard'), `${html} ${key}`).toBe(false);
    }
  });

  it('lets an open modal dialog own all keys, but not a closed one', () => {
    const open = element('<dialog open><button></button></dialog>', 'button');
    expect(ownsKey(open, 'l', 'pointer')).toBe(true);
    const closed = element('<dialog><button></button></dialog>', 'button');
    expect(ownsKey(closed, 'l', 'pointer')).toBe(false);
  });

  it('lets an open popover own every key pressed inside it, but not keys outside it', () => {
    const inside = element('<div role="dialog" data-popover><button>+</button></div><div tabindex="-1" data-home></div>', 'button');
    for (const key of [...LETTERS_AND_TRANSPORT, 'ArrowLeft']) expect(ownsKey(inside, key, 'pointer'), key).toBe(true);
    const home = document.querySelector('[data-home]');
    for (const key of LETTERS_AND_TRANSPORT) expect(ownsKey(home, key, 'pointer'), key).toBe(false);
  });

  it('leaves keys on the page and on plain containers to the shortcuts', () => {
    expect(ownsKey(document.body, ' ', 'keyboard')).toBe(false);
    expect(ownsKey(element('<div tabindex="-1" data-home></div>', 'div'), 'ArrowLeft', 'keyboard')).toBe(false);
    expect(ownsKey(window, ' ', 'keyboard')).toBe(false);
    expect(ownsKey(null, ' ', 'keyboard')).toBe(false);
  });
});
