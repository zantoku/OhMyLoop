import { describe, expect, it, vi } from 'vitest';
import { fireEvent, render, screen, within } from '@testing-library/svelte';
import MapWaveform from './MapWaveform.svelte';
import { timingFixture } from './timing-fixture';

async function pointer(target: Element, type: string, x: number, button = 0) { const event = new MouseEvent(type, { bubbles: true, cancelable: true, clientX: x, button }); Object.defineProperty(event, 'pointerId', { value: 1 }); await fireEvent(target, event); }
function setup(extra: Record<string, unknown> = {}) {
  const callbacks = { onview: vi.fn(), onseek: vi.fn(), onselect: vi.fn(), ongap: vi.fn(), onbeat: vi.fn(), onedit: vi.fn(), ondragging: vi.fn() };
  const result = render(MapWaveform, { peaks: [[-.2, .2]], duration: 12, rate: 1000, position: 1, view: { start: 0, end: 8 }, mapView: timingFixture().view, selected: 'map-3', ...callbacks, ...extra });
  const field = screen.getByRole('region', { name: /^Measure-map waveform/ }), overview = screen.getByRole('button', { name: 'Move measure-map view' });
  for (const element of [field, overview]) {
    vi.spyOn(element, 'getBoundingClientRect').mockReturnValue({ left: 100, width: 1000, height: 200, top: 0, right: 1100, bottom: 200, x: 100, y: 0, toJSON() {} });
    const held = new Set<number>(); element.setPointerCapture = vi.fn(id => { held.add(id); }); element.hasPointerCapture = vi.fn(id => held.has(id)); element.releasePointerCapture = vi.fn(id => { held.delete(id); });
  }
  return { ...callbacks, ...result, field, overview };
}

describe('measure-map waveform', () => {
  it('selects the clicked measure and seeks to the exact point, including shared barlines, and selects free time', async () => {
    const s = setup();
    await pointer(s.field, 'pointerdown', 725); await pointer(s.field, 'pointerup', 725);
    expect(s.onselect).toHaveBeenLastCalledWith('map-5'); expect(s.onseek).toHaveBeenLastCalledWith(5);
    await pointer(s.field, 'pointerdown', 600); await pointer(s.field, 'pointerup', 600);
    expect(s.onselect).toHaveBeenLastCalledWith('map-5'); expect(s.onseek).toHaveBeenLastCalledWith(4);
    expect(s.onedit).not.toHaveBeenCalled();
    const view = timingFixture().view; view.measures[0].end = 3000;
    await s.rerender({ mapView: view }); s.onselect.mockClear();
    await pointer(s.field, 'pointerdown', 550); await pointer(s.field, 'pointerup', 550);
    expect(s.onseek).toHaveBeenLastCalledWith(3.6); expect(s.onselect).not.toHaveBeenCalled(); expect(s.ongap).toHaveBeenLastCalledWith(3600);
  });

  it('draws free time only once a map exists, and marks the chosen range', async () => {
    const s = setup({ mapView: { revision: 0, measures: [] }, selected: null });
    expect(s.field.querySelectorAll('.free')).toHaveLength(0);
    await s.rerender({ mapView: timingFixture().view, view: { start: 0, end: 12 }, freeRange: { start: 8000, end: 12000 } });
    expect(s.field.querySelectorAll('.free')).toHaveLength(1);
    expect(s.field.querySelector('.free')?.classList.contains('chosen')).toBe(true);
  });

  it('thins dotted beat lines and detected marks together and restores detail on zoom', async () => {
    const s = setup(), view = timingFixture().view;
    for (const m of view.measures) m.grid = Array.from({ length: 40 }, (_, i) => ({ frame: m.start + i * 100, quarter: i / 10, label: String(i + 1), id: null }));
    const candidates = Array.from({ length: 201 }, (_, i) => ({ frame: i * 40, downbeat: i % 4 === 0 }));
    await s.rerender({ mapView: view, candidates });
    expect(s.field.querySelectorAll('.beat-line')).toHaveLength(39);
    expect(s.field.querySelectorAll('.detected')).toHaveLength(101);
    expect(s.field.querySelector('.detail')?.textContent).toBe('Every 2 beats · zoom in for detail');
    await s.rerender({ view: { start: 0, end: 2 } });
    expect(s.field.querySelectorAll('.beat-line')).toHaveLength(20);
    expect(s.field.querySelectorAll('.detected')).toHaveLength(51);
    expect(view.measures[0].grid).toHaveLength(40); expect(candidates).toHaveLength(201);
  });

  it('puts drag handles on the selected measure only: grips on its barlines and diamonds on its beats', async () => {
    const s = setup();
    expect(within(s.field).getAllByRole('button', { name: /^Measure \d beat/ }).map(b => b.getAttribute('aria-label'))).toEqual(['Measure 1 beat 2', 'Measure 1 beat 3', 'Measure 1 beat 4']);
    expect([...s.field.querySelectorAll('.grip.current')].map(g => g.getAttribute('aria-label'))).toEqual(['Barline before measure 1', 'Barline before measure 2']);
    // Only the selected measure's handles are Tab stops; the others still drag.
    expect([...s.field.querySelectorAll<HTMLElement>('.grip, .diamond')].filter(h => h.tabIndex === 0).length).toBe(5);
    expect(screen.getByRole('button', { name: 'Barline after measure 2' }).tabIndex).toBe(-1);
    await s.rerender({ selected: 'map-5' });
    expect(within(s.field).getAllByRole('button', { name: /^Measure \d beat/ })[0].getAttribute('aria-label')).toBe('Measure 2 beat 2');
    expect([...s.field.querySelectorAll('.grip.current')].map(g => g.getAttribute('aria-label'))).toEqual(['Barline before measure 2', 'Barline after measure 2']);
  });

  it('captures a barline drag and commits one source-frame edit without seeking, reporting the drag', async () => {
    const s = setup();
    await pointer(screen.getByRole('button', { name: 'Barline before measure 2' }), 'pointerdown', 600);
    expect(s.ondragging).toHaveBeenLastCalledWith(true);
    await pointer(s.field, 'pointermove', 650); expect(s.onedit).not.toHaveBeenCalled();
    await pointer(s.field, 'pointerup', 650);
    expect(s.onedit).toHaveBeenCalledExactlyOnceWith({ type: 'move_boundary', id: 'map-2', frame: 4400 }); expect(s.onseek).not.toHaveBeenCalled();
    expect(s.ondragging).toHaveBeenLastCalledWith(false); expect(s.field.releasePointerCapture).toHaveBeenCalledWith(1);
    // The selected measure's end barline belongs to the next measure, but dragging it keeps the selection.
    expect(s.onselect).toHaveBeenLastCalledWith('map-3');
  });

  it('turns a beat diamond drag into an anchor; arrows nudge it and Delete removes an anchor', async () => {
    const view = timingFixture().view; view.measures[0].grid[2] = { id: 'map-9', frame: 2100, quarter: 2, label: '3' };
    const s = setup({ mapView: view });
    const beat = screen.getByRole('button', { name: 'Measure 1 beat 2' });
    await pointer(beat, 'pointerdown', 225); expect(document.activeElement).toBe(beat); expect(s.onbeat).toHaveBeenLastCalledWith(1);
    await pointer(s.field, 'pointerup', 250);
    expect(s.onedit).toHaveBeenLastCalledWith({ type: 'set_beat', measure: 'map-3', id: null, frame: 1200, offset: { num: 960, den: 960 } });
    await fireEvent.keyDown(beat, { key: 'ArrowRight', shiftKey: true }); expect(s.onedit).toHaveBeenLastCalledWith({ type: 'set_beat', measure: 'map-3', id: null, frame: 1001, offset: { num: 960, den: 960 } });
    s.onedit.mockClear(); await fireEvent.keyDown(beat, { key: 'Delete' }); expect(s.onedit).not.toHaveBeenCalled();
    await fireEvent.keyDown(screen.getByRole('button', { name: 'Measure 1 beat 3, anchored' }), { key: 'Backspace' });
    expect(s.onedit).toHaveBeenLastCalledWith({ type: 'remove_beat', measure: 'map-3', id: 'map-9' });
    await fireEvent.click(beat); expect(s.onseek).not.toHaveBeenCalled();
  });

  it('leaves a locked measure’s handles alone', async () => {
    const view = timingFixture().view; view.measures[0].protected = true;
    const s = setup({ mapView: view });
    const beat = screen.getByRole('button', { name: 'Measure 1 beat 2' });
    expect(beat.getAttribute('aria-disabled')).toBe('true');
    await pointer(beat, 'pointerdown', 225); await pointer(s.field, 'pointerup', 260);
    await fireEvent.keyDown(beat, { key: 'ArrowRight' });
    expect(s.onedit).not.toHaveBeenCalled();
  });

  it('pans and zooms without map edits, cancels pointer gestures, and clamps minimap navigation', async () => {
    const s = setup(); await pointer(s.field, 'pointerdown', 600, 1); await pointer(s.field, 'pointermove', 700, 1); expect(s.onview).toHaveBeenLastCalledWith({ start: 0, end: 8 }); await pointer(s.field, 'pointercancel', 700, 1); expect(s.onedit).not.toHaveBeenCalled(); expect(s.onseek).not.toHaveBeenCalled();
    const wheel = new WheelEvent('wheel', { bubbles: true, cancelable: true, clientX: 350, deltaY: -100 }); await fireEvent(s.field, wheel); expect(wheel.defaultPrevented).toBe(true); expect(s.onview.mock.lastCall?.[0].end - s.onview.mock.lastCall?.[0].start).toBeCloseTo(8 * Math.exp(-.25));
    await pointer(s.overview, 'pointerdown', 900); await pointer(s.overview, 'pointermove', 2000); await pointer(s.overview, 'pointerup', 2000); expect(s.onview).toHaveBeenLastCalledWith({ start: 4, end: 12 }); expect(s.onedit).not.toHaveBeenCalled();
  });

  it('shows a draft in green and the split preview line', async () => {
    const s = setup({ draft: true, splitAt: 2000 });
    expect(s.field.classList.contains('draft')).toBe(true);
    expect((s.field.querySelector('.split-preview') as HTMLElement).style.left).toBe('25%');
  });
});
