import { describe, expect, it } from 'vitest';
import { laneRows, nestedSections } from './section-layout';

const section = (id: string, start: number, end: number) => ({ id, name: id, start, end });

describe('section layout', () => {
  it('indents sections that lie inside another section', () => {
    const solo = section('solo', 33, 48), lick = section('lick', 37, 38), verse = section('verse', 9, 24);
    expect(nestedSections([verse, solo, lick])).toEqual(new Set(['lick']));
    // Identical bounds: only the later one is indented.
    expect(nestedSections([section('a', 0, 4), section('b', 0, 4)])).toEqual(new Set(['b']));
  });
  it('stacks overlapping sections in lane rows and reuses free rows', () => {
    const rows = laneRows([section('verse', 0, 10), section('solo', 10, 40), section('lick', 20, 22), section('outro', 40, 50)]);
    expect(Object.fromEntries(rows)).toEqual({ verse: 0, solo: 0, lick: 1, outro: 0 });
  });
  it('caps the lane at three rows', () => {
    const rows = laneRows([0, 1, 2, 3].map(i => section(`s${i}`, i, 10)));
    expect(Math.max(...rows.values())).toBe(2);
  });
});
