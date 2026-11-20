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
    const [toast] = notices({ ...calm, processing: { id: 'job', kind: 'running', title: 'Splitting song into stems', detail: 'Playback is paused until it finishes.', progress: .58 } });
    expect(toast).toMatchObject({ tone: 'progress', title: 'Splitting song into stems · 58 %', progress: .58, action: { button: 'separation_open' } });
    expect(toast.text).toMatch(/Playback is paused/);
    expect(toast.dismiss).toBeUndefined();
    const [unmeasured] = notices({ ...calm, processing: { id: 'setup', kind: 'running', title: 'Setting up stem separation', detail: 'Installing separation tools. Playback keeps working.', progress: null } });
    expect(unmeasured).toMatchObject({ title: 'Setting up stem separation…', progress: null });
  });

  it('keeps a finished or failed separation as a dismissible toast that opens Stems', () => {
    const [done] = notices({ ...calm, processing: { id: 'job', kind: 'done', title: 'Stems ready', detail: 'Five instruments; piano was near-silent and left out.', progress: null } });
    expect(done).toMatchObject({ tone: 'success', title: 'Stems ready', dismiss: 'hide', action: { button: 'separation_open', label: 'Open Stems' } });
    const [failed] = notices({ ...calm, processing: { id: 'job', kind: 'failed', title: 'Separation stopped', detail: 'Not enough memory.', progress: null } });
    expect(failed).toMatchObject({ tone: 'error', title: 'Separation stopped', text: 'Not enough memory.', dismiss: 'hide', action: { button: 'separation_open' } });
  });

  it('confirms a Measure map split with Undo, once per split, and not while work runs', () => {
    const split = { id: 1, text: 'Split m. 37 before beat 3. Later measures renumbered.' };
    expect(notices({ ...calm, mapNotice: split })).toEqual([{ key: `map-1:${split.text}`, tone: 'info', text: split.text, action: { button: 'map_undo', label: 'Undo', enabled: true }, dismiss: 'hide' }]);
    expect(notices({ ...calm, mapNotice: split })[0].key).not.toBe(notices({ ...calm, mapNotice: { ...split, id: 2 } })[0].key);
    expect(notices({ ...calm, mapNotice: split, busy: 'Saving measure map…' }).map(n => n.tone)).toEqual(['progress']);
  });
  it('offers Undo for each removal, but not while work is running', () => {
    expect(notices({ ...calm, undoCount: 1 })[0]).toMatchObject({ tone: 'info', action: { button: 'undo_delete' } });
    expect(notices({ ...calm, undoCount: 1 })[0].key).not.toBe(notices({ ...calm, undoCount: 2 })[0].key);
    expect(notices({ ...calm, undoCount: 1, busy: 'Opening song…' }).map(n => n.tone)).toEqual(['progress']);
  });
});
