import { beforeEach, describe, expect, it, vi } from 'vitest';
import { fireEvent, render, screen } from '@testing-library/svelte';
import ThemeControl from './ThemeControl.svelte';
import { readThemePreference } from './theme';

beforeEach(() => {
  localStorage.clear();
  delete document.documentElement.dataset.theme;
});

describe('UI appearance preference', () => {
  it('defaults to auto and persists the cycle across remounts', async () => {
    const first = render(ThemeControl);
    const auto = screen.getByRole('button', { name: 'UI: auto' });
    expect(document.documentElement.dataset.theme).toBe('auto');
    await fireEvent.click(auto);
    expect(document.documentElement.dataset.theme).toBe('dark');
    expect(readThemePreference()).toBe('dark');
    first.unmount();

    render(ThemeControl);
    await fireEvent.click(screen.getByRole('button', { name: 'UI: dark' }));
    expect(document.documentElement.dataset.theme).toBe('light');
    expect(readThemePreference()).toBe('light');
    await fireEvent.click(screen.getByRole('button', { name: 'UI: light' }));
    expect(document.documentElement.dataset.theme).toBe('auto');
    expect(readThemePreference()).toBe('auto');
  });

  it('falls back to OS appearance without overwriting an invalid stored value', () => {
    localStorage.setItem('ohmyloop.ui.theme', 'future-theme');
    render(ThemeControl);
    expect(screen.getByRole('button', { name: 'UI: auto' })).toBeTruthy();
    expect(document.documentElement.dataset.theme).toBe('auto');
    expect(localStorage.getItem('ohmyloop.ui.theme')).toBe('future-theme');
  });

  it('still changes appearance and reports a failed preference save', async () => {
    const write = vi.spyOn(Storage.prototype, 'setItem').mockImplementation(() => { throw new Error('Storage unavailable'); });
    try {
      render(ThemeControl);
      await fireEvent.click(screen.getByRole('button', { name: 'UI: auto' }));
      expect(document.documentElement.dataset.theme).toBe('dark');
      expect(screen.getByRole('status').textContent).toBe('UI preference could not be saved.');
    } finally {
      write.mockRestore();
    }
  });

  it('uses auto when preference storage cannot be read', () => {
    const read = vi.spyOn(Storage.prototype, 'getItem').mockImplementation(() => { throw new Error('Storage unavailable'); });
    try {
      expect(readThemePreference()).toBe('auto');
    } finally {
      read.mockRestore();
    }
  });
});
