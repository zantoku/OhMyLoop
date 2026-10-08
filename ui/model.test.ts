import { describe, it, expect } from 'vitest';
import { editing, ordered, range, time, zoom } from './model';

describe('waveform interaction coordinates', () => {
  it('selects the same source range when dragging backwards', () => {
    expect(range(8, 3, 12)).toEqual(range(3, 8, 12));
    expect(range(.2, .3, 12)).not.toBeNull();
    expect(range(2, 2.02, 12)).toBeNull();
  });
  it('keeps zoom centered on a phrase and inside the source', () => {
    expect(zoom({ start: 0, end: 12 }, 12, .5, 8)).toEqual({ start: 5, end: 11 });
    expect(zoom({ start: 0, end: 12 }, 12, .5, .1)).toEqual({ start: 0, end: 6 });
  });
  it('anchors wheel zoom at the pointer and clamps at both ends and zoom limits', () => {
    const view = { start: 2, end: 10 };
    expect(zoom(view, 20, .5, 4, .25)).toEqual({ start: 3, end: 7 });
    expect(zoom(view, 20, .5, 2, 0)).toEqual({ start: 2, end: 6 });
    expect(zoom(view, 20, .5, 10, 1)).toEqual({ start: 6, end: 10 });
    expect(zoom(view, 20, 4, 4, .25)).toEqual({ start: 0, end: 20 });
    expect(zoom({ start: 0, end: 1 }, 20, .001, 0, 0)).toEqual({ start: 0, end: .25 });
    expect(zoom({ start: 19, end: 20 }, 20, 2, 20, 0)).toEqual({ start: 18, end: 20 });
  });
  it('navigates nested sections without changing their saved order or identity', () => {
    const sections = [{ id: 'phrase', name: 'Phrase', start: 4, end: 5 }, { id: 'verse', name: 'Verse', start: 0, end: 8 }];
    expect(ordered(sections).map(s => s.id)).toEqual(['verse', 'phrase']);
    expect(sections[0].id).toBe('phrase');
  });
  it('formats minute boundaries without showing 0:60.0', () => {
    expect(time(59.99)).toBe('1:00.0');
  });
  it('does not fire transport shortcuts while editing numeric controls or dialogs', () => {
    const input = document.createElement('input'), button = document.createElement('button'), dialog = document.createElement('dialog');
    dialog.append(button);
    expect(editing(input)).toBe(true); expect(editing(button)).toBe(true); expect(editing(document.body)).toBe(false);
  });
});
