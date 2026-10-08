import { describe, expect, it, vi } from 'vitest';
import { fireEvent, render, screen } from '@testing-library/svelte';
import Waveform from './Waveform.svelte';
import type { View } from './types';

function setup() {
  const callbacks = { onview: vi.fn<(view: View) => void>(), onseek: vi.fn(), onselection: vi.fn(), onsection: vi.fn() };
  const props = { peaks: [[-.2, .2]] as [number, number][], duration: 20, sections: [],
    selection: { start: 4, end: 5, fade_ms: 5 }, position: 3, view: { start: 2, end: 10 }, ...callbacks };
  const component = render(Waveform, props);
  const field = screen.getByRole('region', { name: /^Waveform/ });
  vi.spyOn(field, 'getBoundingClientRect').mockReturnValue({ left: 100, top: 0, width: 1000, height: 200, right: 1100, bottom: 200, x: 100, y: 0, toJSON() {} });
  const capture = new Set<number>();
  field.setPointerCapture = vi.fn(id => { capture.add(id); });
  field.hasPointerCapture = vi.fn(id => capture.has(id));
  field.releasePointerCapture = vi.fn(id => { capture.delete(id); });
  return { ...callbacks, ...component, field };
}

// jsdom has MouseEvent but no PointerEvent; add the pointer ID used by capture.
async function pointer(target: Element, type: string, clientX: number, button = 0) {
  const event = new MouseEvent(type, { bubbles: true, cancelable: true, clientX, button });
  Object.defineProperty(event, 'pointerId', { value: 1 });
  await fireEvent(target, event);
  return event;
}

describe('waveform navigation gestures', () => {
  it('zooms at the pointer, normalizes wheel units, and cancels page scrolling', async () => {
    const { field, onview, onseek, onselection } = setup();
    const wheel = new WheelEvent('wheel', { bubbles: true, cancelable: true, clientX: 350, deltaY: -100 });
    await fireEvent(field, wheel);
    const view = onview.mock.calls[0][0];
    expect(view.end - view.start).toBeCloseTo(8 * Math.exp(-.25));
    expect(view.start + (view.end - view.start) * .25).toBeCloseTo(4);
    expect(wheel.defaultPrevented).toBe(true);
    await fireEvent.wheel(field, { clientX: 350, deltaY: -6.25, deltaMode: 1 });
    expect(onview.mock.calls[1][0]).toEqual(view);
    await fireEvent.wheel(field, { clientX: 350, deltaY: -.5, deltaMode: 2 });
    expect(onview.mock.calls[2][0]).toEqual(view);
    expect(onseek).not.toHaveBeenCalled(); expect(onselection).not.toHaveBeenCalled();
  });

  it('middle-drags from the initial viewport even as view props update, without editing a loop', async () => {
    const { field, rerender, onview, onseek, onselection } = setup();
    const down = await pointer(screen.getByRole('button', { name: 'Loop start' }), 'pointerdown', 350, 1);
    expect(down.defaultPrevented).toBe(true);
    expect(field.classList.contains('panning')).toBe(true);
    await pointer(field, 'pointermove', 450, 1);
    expect(onview).toHaveBeenLastCalledWith({ start: 1.2, end: 9.2 });
    await rerender({ view: onview.mock.calls[0][0] });
    await pointer(field, 'pointermove', 550, 1);
    expect(onview.mock.lastCall?.[0].start).toBeCloseTo(.4);
    expect(onview.mock.lastCall?.[0].end).toBeCloseTo(8.4);
    await pointer(field, 'pointermove', 1000, 1);
    expect(onview).toHaveBeenLastCalledWith({ start: 0, end: 8 });
    await pointer(field, 'pointermove', -2500, 1);
    expect(onview).toHaveBeenLastCalledWith({ start: 12, end: 20 });
    await pointer(field, 'pointerup', -2500, 1);
    expect(field.classList.contains('panning')).toBe(false);
    expect(field.releasePointerCapture).toHaveBeenCalledWith(1);
    expect(onseek).not.toHaveBeenCalled(); expect(onselection).not.toHaveBeenCalled();
  });

  it('does not seek on a middle click and releases a canceled drag', async () => {
    const { field, onview, onseek, onselection } = setup();
    await pointer(field, 'pointerdown', 350, 1);
    await pointer(field, 'pointerup', 350, 1);
    await pointer(field, 'pointerdown', 350, 1);
    await fireEvent.wheel(field, { clientX: 350, deltaY: -100 });
    expect(onview).not.toHaveBeenCalled();
    await pointer(field, 'pointercancel', 350, 1);
    await pointer(field, 'pointermove', 450, 1);
    expect(field.classList.contains('panning')).toBe(false);
    expect(field.hasPointerCapture(1)).toBe(false);
    expect(onview).not.toHaveBeenCalled(); expect(onseek).not.toHaveBeenCalled(); expect(onselection).not.toHaveBeenCalled();
  });

  it('preserves left-button selection and seeking', async () => {
    const { field, onview, onseek, onselection } = setup();
    await pointer(field, 'pointerdown', 350);
    await pointer(field, 'pointermove', 750);
    await pointer(field, 'pointerup', 750);
    expect(onselection).toHaveBeenCalledWith({ start: 4, end: 7.2, fade_ms: 5 });
    await pointer(field, 'pointerdown', 850);
    await pointer(field, 'pointerup', 850);
    expect(onseek).toHaveBeenCalledWith(8);
    expect(onview).not.toHaveBeenCalled();
  });
});
