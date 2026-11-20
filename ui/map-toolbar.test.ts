import { describe, expect, it } from 'vitest';
import { placeToolbar } from './map-toolbar';

const field = 1000, width = 400;
describe('floating toolbar placement', () => {
  it('centres on a measure in view, with the notch in the middle', () => {
    expect(placeToolbar({ start: 400, end: 600, field, width })).toEqual({ kind: 'bar', left: 300, notch: 200 });
  });

  it('stays inside the waveform against an edge and slides the notch to the measure', () => {
    // Near the right edge: the toolbar stops at the margin; the notch moves right.
    const right = placeToolbar({ start: 880, end: 990, field, width });
    expect(right).toEqual({ kind: 'bar', left: 594, notch: 935 - 594 });
    // Partly scrolled off the left: the notch points at the visible part only.
    const left = placeToolbar({ start: -150, end: 60, field, width });
    expect(left).toEqual({ kind: 'bar', left: 6, notch: 24 });
    const partly = placeToolbar({ start: -50, end: 120, field, width });
    expect(partly.kind === 'bar' && partly.left).toBe(6);
    expect(partly.kind === 'bar' && partly.notch).toBe(54);
  });

  it('keeps the notch clear of the rounded corners', () => {
    const edge = placeToolbar({ start: 990, end: 1100, field, width });
    expect(edge).toEqual({ kind: 'bar', left: 594, notch: width - 16 });
  });

  it('collapses to an edge tab on the side the measure went once it is out of view', () => {
    expect(placeToolbar({ start: -300, end: -20, field, width })).toEqual({ kind: 'tab', side: 'left' });
    expect(placeToolbar({ start: 1010, end: 1200, field, width })).toEqual({ kind: 'tab', side: 'right' });
    // A sliver of a pixel at an edge is not "visible".
    expect(placeToolbar({ start: -300, end: 1, field, width })).toEqual({ kind: 'tab', side: 'left' });
    expect(placeToolbar({ start: 999, end: 1300, field, width })).toEqual({ kind: 'tab', side: 'right' });
  });

  it('keeps the toolbar for a measure narrower than 2 px inside the view, pointing at its centre', () => {
    expect(placeToolbar({ start: 500, end: 501, field, width })).toEqual({ kind: 'bar', left: 300.5, notch: 200 });
    expect(placeToolbar({ start: 20, end: 21, field, width })).toEqual({ kind: 'bar', left: 6, notch: 16 });
    expect(placeToolbar({ start: 980, end: 980.5, field, width })).toEqual({ kind: 'bar', left: 594, notch: 384 });
    expect(placeToolbar({ start: 600, end: 600, field, width })).toEqual({ kind: 'bar', left: 400, notch: 200 });
  });

  it('pins a toolbar wider than the field to the left margin', () => {
    expect(placeToolbar({ start: 100, end: 200, field: 300, width: 400 })).toEqual({ kind: 'bar', left: 6, notch: 144 });
  });

  it('handles a measure wider than the view, pointing at the middle of the view', () => {
    expect(placeToolbar({ start: -500, end: 1500, field, width })).toEqual({ kind: 'bar', left: 300, notch: 200 });
  });
});
