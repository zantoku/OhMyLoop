// Where the Measure map's floating toolbar sits (concept MB, measure-map.md).
// Pure geometry in the waveform's pixel space: the toolbar centres on the
// selected measure, stays fully inside the waveform against an edge, and its
// notch slides along its top edge to point at the visible part of the measure.
// When the measure has scrolled out of view the toolbar collapses to an edge
// tab on the side the measure went. It never stays open for a hidden measure.

export type ToolbarPlacement =
  /** `left` is the toolbar's left edge in the field; `notch` is measured from that edge. */
  | { kind: 'bar'; left: number; notch: number }
  | { kind: 'tab'; side: 'left' | 'right' };

export type ToolbarGeometry = {
  /** The selected range's start and end in field pixels; either may lie outside the field. */
  start: number; end: number;
  /** Field width and the toolbar's measured width, in pixels. */
  field: number; width: number;
  /** Gap kept between the toolbar and the field's edges. */
  margin?: number;
  /** How close the notch may come to the toolbar's rounded corners. */
  inset?: number;
};

/** A range within this many pixels of the far side of an edge counts as out of view. */
export const MIN_VISIBLE = 2;

export function placeToolbar({ start, end, field, width, margin = 6, inset = 16 }: ToolbarGeometry): ToolbarPlacement {
  const low = Math.min(start, end), high = Math.max(start, end);
  // Out of view: it ends before the left edge or starts after the right one.
  if (high < MIN_VISIBLE) return { kind: 'tab', side: 'left' };
  if (low > field - MIN_VISIBLE) return { kind: 'tab', side: 'right' };
  // A sliver inside the view (a dense map) still gets the toolbar.
  const from = Math.max(0, low), to = Math.min(field, high);
  const centre = (start + end) / 2;
  const room = Math.max(0, field - width - 2 * margin);
  const left = margin + Math.min(room, Math.max(0, centre - width / 2 - margin));
  // The notch points at the middle of what's visible, kept clear of the corners.
  const target = (from + to) / 2 - left;
  const notch = width <= 2 * inset ? width / 2 : Math.min(width - inset, Math.max(inset, target));
  return { kind: 'bar', left, notch };
}
