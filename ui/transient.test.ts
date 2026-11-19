import { describe, expect, it, vi } from 'vitest';
import { closeTopTransient, openTransient } from './transient';

describe('transient UI unwinds newest first', () => {
  it('closes one level per call and reports when nothing is open', () => {
    const outer = vi.fn(), inner = vi.fn();
    openTransient(outer); openTransient(inner);
    expect(closeTopTransient()).toBe(true);
    expect(inner).toHaveBeenCalledOnce(); expect(outer).not.toHaveBeenCalled();
    expect(closeTopTransient()).toBe(true);
    expect(outer).toHaveBeenCalledOnce();
    expect(closeTopTransient()).toBe(false);
  });

  it('forgets a popover that closed by other means', () => {
    const close = vi.fn();
    const release = openTransient(close);
    release();
    expect(closeTopTransient()).toBe(false);
    expect(close).not.toHaveBeenCalled();
  });
});
