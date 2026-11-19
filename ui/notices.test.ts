import { describe, expect, it } from 'vitest';
import { notices, type NoticeState } from './notices';

const calm: NoticeState = { error: '', startupNotice: '', busy: '', preview: false, undoCount: 0, audioError: null, audioRetry: true, saveError: null, keymapWarning: null, keymapLoading: false,
  stemMixError: '', stemMixRetry: false, stemWarning: null, stemsUnavailable: false, countInError: '', clickError: '', processing: null };

describe('notices', () => {
  it('shows nothing when all is well', () => expect(notices(calm)).toEqual([]));

  it('lists several errors at once, each with its recovery action, before informational notices', () => {
    const list = notices({ ...calm, audioError: 'Device disconnected', saveError: 'Disk full', keymapWarning: 'Invalid keymap', startupNotice: 'Library moved' });
    expect(list.map(n => [n.tone, n.action?.button])).toEqual([['error', 'retry_audio'], ['error', 'retry_save'], ['error', 'reload_keymap'], ['info', undefined]]);
    expect(list[0]).toMatchObject({ title: 'Playback stopped.', dismiss: 'hide' });
  });

  it('dismisses the app error through its action and keys messages by text', () => {
    const [first] = notices({ ...calm, error: 'One' }), [second] = notices({ ...calm, error: 'Two' });
    expect(first.dismiss).toBe('dismiss_error');
    expect(first.key).not.toBe(second.key);
  });

  it('reports separation progress with a way to open Stems, and why playback is paused', () => {
    const [toast] = notices({ ...calm, processing: { label: 'Splitting song into stems', detail: 'Playback is paused until it finishes. Notes still work.', progress: .58 } });
    expect(toast).toMatchObject({ tone: 'progress', title: 'Splitting song into stems · 58 %', progress: .58, action: { button: 'separation_open' } });
    expect(toast.text).toMatch(/Playback is paused/);
    expect(toast.dismiss).toBeUndefined();
  });

  it('offers Undo for each removal, but not while work is running', () => {
    expect(notices({ ...calm, undoCount: 1 })[0]).toMatchObject({ tone: 'info', action: { button: 'undo_delete' } });
    expect(notices({ ...calm, undoCount: 1 })[0].key).not.toBe(notices({ ...calm, undoCount: 2 })[0].key);
    expect(notices({ ...calm, undoCount: 1, busy: 'Opening song…' }).map(n => n.tone)).toEqual(['progress']);
  });
});
