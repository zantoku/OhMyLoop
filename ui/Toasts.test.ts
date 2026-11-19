import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { fireEvent, render, screen } from '@testing-library/svelte';
import Toasts from './Toasts.svelte';
import { Keymap } from './keymap';
import { INFO_TIMEOUT_MS, type Notice } from './notices';

const info: Notice = { key: 'startup:Moved', tone: 'info', text: 'Library moved', dismiss: 'hide' };
const error: Notice = { key: 'error:Disk full', tone: 'error', text: 'Disk full', dismiss: 'dismiss_error' };
const audio: Notice = { key: 'audio:Lost', tone: 'error', title: 'Playback stopped.', text: 'Lost', action: { button: 'retry_audio', label: 'Retry audio', enabled: true }, dismiss: 'hide' };

describe('toasts', () => {
  beforeEach(() => vi.useFakeTimers());
  afterEach(() => vi.useRealTimers());

  it('shows several at once, runs recovery actions and dismisses the app error through its action', async () => {
    const run = vi.fn();
    render(Toasts, { notices: [audio, error], keymap: new Keymap(), run });
    expect(screen.getAllByRole('alert')).toHaveLength(2);
    await fireEvent.click(screen.getByRole('button', { name: 'Retry audio' }));
    expect(run).toHaveBeenCalledWith('retry_audio');
    await fireEvent.click(screen.getAllByRole('button', { name: 'Dismiss' })[1]);
    expect(run).toHaveBeenLastCalledWith('dismiss_error');
    expect(screen.getAllByRole('alert')).toHaveLength(2); // The app clears its error.
  });

  it('times out informational notices but keeps errors until resolved or dismissed', async () => {
    render(Toasts, { notices: [info, audio], keymap: new Keymap(), run: vi.fn() });
    expect(screen.getByText('Library moved')).toBeTruthy();
    await vi.advanceTimersByTimeAsync(INFO_TIMEOUT_MS);
    expect(screen.queryByText('Library moved')).toBeNull();
    expect(screen.getByRole('alert').textContent).toContain('Playback stopped.');
    await fireEvent.click(screen.getByRole('button', { name: 'Dismiss' }));
    expect(screen.queryByRole('alert')).toBeNull();
  });

  it('shows a dismissed message again when it recurs after being resolved', async () => {
    const view = render(Toasts, { notices: [audio], keymap: new Keymap(), run: vi.fn() });
    await fireEvent.click(screen.getByRole('button', { name: 'Dismiss' }));
    await view.rerender({ notices: [] });
    await view.rerender({ notices: [audio] });
    expect(screen.getByRole('alert')).toBeTruthy();
  });

  it('shows progress without a dismiss button', () => {
    render(Toasts, { notices: [{ key: 'p', tone: 'progress', title: 'Splitting song into stems · 58 %', text: 'Playback is paused.', progress: .58 }], keymap: new Keymap(), run: vi.fn() });
    expect(screen.getByRole('status').textContent).toContain('58 %');
    expect(screen.queryByRole('button', { name: 'Dismiss' })).toBeNull();
  });
});
