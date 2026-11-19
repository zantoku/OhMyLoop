import { describe, expect, it } from 'vitest';
import { ownsKey } from './key-routing';
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

  it('leaves keys on the page and on plain containers to the shortcuts', () => {
    expect(ownsKey(document.body, ' ', 'keyboard')).toBe(false);
    expect(ownsKey(element('<div tabindex="-1" data-home></div>', 'div'), 'ArrowLeft', 'keyboard')).toBe(false);
    expect(ownsKey(window, ' ', 'keyboard')).toBe(false);
    expect(ownsKey(null, ' ', 'keyboard')).toBe(false);
  });
});
