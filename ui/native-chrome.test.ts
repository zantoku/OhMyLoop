import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { allowsContextMenu, blocksDrag, installNativeChrome, keepsFocusOnPress } from './native-chrome';
import { currentModality, installInputModality } from './input-modality';
import { atHome, focusHome } from './home-focus';
import { installFieldCommit } from './field-commit';

function mount(html: string): HTMLElement {
  document.body.innerHTML = html;
  return document.body;
}
const cleanups: (() => void)[] = [];
afterEach(() => { cleanups.splice(0).forEach(cleanup => cleanup()); document.body.innerHTML = ''; delete document.documentElement.dataset.modality; });

describe('clicks leave focus where it was', () => {
  it('keeps focus off buttons, toggles, tabs and links, but not off fields or composite controls', () => {
    const page = mount(`<button id="b"><svg></svg></button><input id="c" type="checkbox"><div id="t" role="tab"></div><a id="a" href="#">x</a>
      <label id="lc"><input type="checkbox">Snap</label><label id="lt">Name<input id="field"></label>
      <select id="s"></select><input id="r" type="range"><div id="k" role="slider" tabindex="0"></div><button id="h" data-owns-keys="ArrowLeft">A</button><p id="p">text</p>`);
    const at = (id: string) => page.querySelector(`#${id}`);
    for (const id of ['b', 'c', 't', 'a', 'lc']) expect(keepsFocusOnPress(at(id)), id).toBe(true);
    expect(keepsFocusOnPress(page.querySelector('#b svg'))).toBe(true);
    for (const id of ['lt', 'field', 's', 'r', 'k', 'h', 'p']) expect(keepsFocusOnPress(at(id)), id).toBe(false);
  });

  it('commits a focused field before cancelling a press on a button', () => {
    const home = vi.fn(() => (document.activeElement as HTMLElement).blur());
    cleanups.push(installNativeChrome(document, home));
    const page = mount('<input type="number" value="1"><button>Save</button>');
    const field = page.querySelector('input')!; field.focus();
    const press = new MouseEvent('mousedown', { bubbles: true, cancelable: true, button: 0 });
    page.querySelector('button')!.dispatchEvent(press);
    expect(home).toHaveBeenCalledOnce();
    expect(document.activeElement).not.toBe(field);
    expect(press.defaultPrevented).toBe(true);
    // A press inside the focused field itself does not commit it.
    field.focus(); home.mockClear();
    field.dispatchEvent(new MouseEvent('mousedown', { bubbles: true, cancelable: true, button: 0 }));
    expect(home).not.toHaveBeenCalled();
  });

  it('cancels the press on a button so a focused workspace keeps focus', () => {
    cleanups.push(installNativeChrome());
    const page = mount('<div id="home" tabindex="-1"></div><button>Loop</button><input>');
    const press = new MouseEvent('mousedown', { bubbles: true, cancelable: true, button: 0 });
    page.querySelector('button')!.dispatchEvent(press);
    expect(press.defaultPrevented).toBe(true);
    const field = new MouseEvent('mousedown', { bubbles: true, cancelable: true, button: 0 });
    page.querySelector('input')!.dispatchEvent(field);
    expect(field.defaultPrevented).toBe(false);
  });
});

describe('browser chrome', () => {
  it('suppresses the context menu except in text fields', () => {
    cleanups.push(installNativeChrome());
    const page = mount('<p>Label</p><input><textarea></textarea>');
    const menu = (selector: string) => { const event = new MouseEvent('contextmenu', { bubbles: true, cancelable: true }); page.querySelector(selector)!.dispatchEvent(event); return event.defaultPrevented; };
    expect(menu('p')).toBe(true);
    expect(menu('input')).toBe(false);
    expect(menu('textarea')).toBe(false);
    expect(allowsContextMenu(page.querySelector('input[type=checkbox]'))).toBe(false);
  });

  it('blocks dragging images, SVGs and links unless a drag is designed', () => {
    const page = mount('<img><svg><path></path></svg><a href="#"><span>x</span></a><div draggable="true"><img id="d"></div><p>t</p>');
    expect(blocksDrag(page.querySelector('img'))).toBe(true);
    expect(blocksDrag(page.querySelector('path'))).toBe(true);
    expect(blocksDrag(page.querySelector('a span'))).toBe(true);
    expect(blocksDrag(page.querySelector('#d'))).toBe(false);
    expect(blocksDrag(page.querySelector('p'))).toBe(false);
  });
});

describe('input modality', () => {
  it('switches to keyboard on Tab or arrows and back to pointer on a press', () => {
    cleanups.push(installInputModality());
    expect(currentModality()).toBe('pointer');
    document.dispatchEvent(new KeyboardEvent('keydown', { key: 'l' }));
    expect(currentModality()).toBe('pointer');
    document.dispatchEvent(new KeyboardEvent('keydown', { key: 'Tab' }));
    expect(document.documentElement.dataset.modality).toBe('keyboard');
    document.dispatchEvent(new Event('pointerdown'));
    expect(currentModality()).toBe('pointer');
    document.dispatchEvent(new KeyboardEvent('keydown', { key: 'ArrowDown' }));
    expect(currentModality()).toBe('keyboard');
  });
});

describe('home focus', () => {
  it('focuses the visible workspace, or clears focus when the screen has none', () => {
    const page = mount('<div hidden><div id="hidden" tabindex="-1" data-home></div></div><div id="home" tabindex="-1" data-home></div><button>Play</button>');
    page.querySelector('button')!.focus();
    expect(atHome()).toBe(false);
    focusHome();
    expect(document.activeElement?.id).toBe('home');
    expect(atHome()).toBe(true);
    mount('<input>');
    document.querySelector('input')!.focus();
    focusHome();
    expect(document.activeElement).toBe(document.body);
  });
});

describe('committing a field hands focus back', () => {
  let home: ReturnType<typeof vi.fn<() => void>>;
  beforeEach(() => { home = vi.fn<() => void>(() => (document.activeElement as HTMLElement | null)?.blur()); cleanups.push(installFieldCommit(document, home)); });
  const key = (target: Element, key: string) => { const event = new KeyboardEvent('keydown', { key, bubbles: true, cancelable: true }); target.dispatchEvent(event); return event; };

  it('commits on Enter and returns home', () => {
    const field = mount('<input type="number" value="80">').querySelector('input')!;
    const change = vi.fn(); field.addEventListener('change', change);
    field.focus(); field.value = '90';
    expect(key(field, 'Enter').defaultPrevented).toBe(true);
    expect(home).toHaveBeenCalledOnce();
    expect(field.value).toBe('90');
    // Enter does not commit by itself: the blur from returning home does.
    expect(change).not.toHaveBeenCalled();
    expect(document.activeElement).not.toBe(field);
  });

  it('restores the focused value on Esc, notifies bindings and returns home', () => {
    const field = mount('<input value="Verse">').querySelector('input')!;
    const input = vi.fn(); field.addEventListener('input', input);
    field.focus(); field.value = 'Chorus';
    expect(key(field, 'Escape').defaultPrevented).toBe(true);
    expect(field.value).toBe('Verse');
    expect(input).toHaveBeenCalledOnce();
    expect(home).toHaveBeenCalledOnce();
  });

  it('returns home from a select on Esc', () => {
    const select = mount('<select><option>1</option></select>').querySelector('select')!;
    select.focus();
    expect(key(select, 'Escape').defaultPrevented).toBe(true);
    expect(home).toHaveBeenCalledOnce();
  });

  it('leaves dialogs, forms, textareas and checkboxes to their own behaviour', () => {
    const page = mount('<dialog open><input id="d"></dialog><form><input id="f"></form><textarea></textarea><input id="c" type="checkbox">');
    for (const selector of ['#d', '#f', 'textarea', '#c']) expect(key(page.querySelector(selector)!, 'Enter').defaultPrevented, selector).toBe(false);
    for (const selector of ['#d', 'textarea', '#c']) expect(key(page.querySelector(selector)!, 'Escape').defaultPrevented, selector).toBe(false);
    expect(home).not.toHaveBeenCalled();
  });
});
