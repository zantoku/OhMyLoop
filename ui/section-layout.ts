// Layout of saved sections, which may overlap or nest: indentation in the
// sections list and stacked rows in the timeline lane.
import type { Section } from './types';

/** Sections lying inside another, longer or earlier-listed, section. */
export function nestedSections(sections: readonly Section[]): Set<string> {
  return new Set(sections.filter((inner, i) => sections.some((outer, j) => j !== i &&
    outer.start <= inner.start && inner.end <= outer.end &&
    (outer.end - outer.start > inner.end - inner.start || j < i))).map(section => section.id));
}

/** Packs overlapping sections into rows; a section takes the first row it fits. Rows past `max` share the last row. */
export function laneRows(sections: readonly Section[], max = 3): Map<string, number> {
  const ends: number[] = [], rows = new Map<string, number>();
  for (const section of [...sections].sort((a, b) => a.start - b.start || b.end - a.end)) {
    let row = ends.findIndex(end => end <= section.start);
    if (row < 0) row = ends.length < max ? ends.length : max - 1;
    ends[row] = Math.max(ends[row] ?? 0, section.end);
    rows.set(section.id, row);
  }
  return rows;
}
