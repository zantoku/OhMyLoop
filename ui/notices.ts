// Global notices shown as toasts. Errors that need action persist until they
// are resolved or dismissed; informational notices time out; progress notices
// stay while their work runs. This module derives the list from app state.
import type { ButtonId } from './actions';

export type NoticeTone = 'error' | 'info' | 'progress';
export type Notice = {
  /** Stable per message: a new text shows again after a dismissal. */
  key: string;
  tone: NoticeTone;
  title?: string;
  text: string;
  action?: { button: ButtonId; label: string; enabled: boolean };
  /** `dismiss_error` clears the app error; other dismissals only hide the toast. */
  dismiss?: ButtonId | 'hide';
  /** 0–1 for determinate progress, null for indeterminate. */
  progress?: number | null;
};

export const INFO_TIMEOUT_MS = 6000;

export type NoticeState = {
  error: string; startupNotice: string; busy: string; preview: boolean;
  /** Removed sections that Undo can restore. */
  undoCount: number;
  audioError: string | null; audioRetry: boolean; saveError: string | null; keymapWarning: string | null; keymapLoading: boolean;
  stemMixError: string; stemMixRetry: boolean; stemWarning: string | null; stemsUnavailable: boolean;
  countInError: string; clickError: string;
  /** A passing explanation, such as why Settings can't open right now. */
  info?: string;
  /** Running separation or setup, shown outside the Stems screen. */
  processing: { label: string; detail: string; progress: number | null } | null;
};

const key = (id: string, text: string) => `${id}:${text}`;

export function notices(state: NoticeState): Notice[] {
  const list: Notice[] = [];
  const add = (id: string, notice: Omit<Notice, 'key'>) => list.push({ key: key(id, notice.text), ...notice });
  if (state.audioError) add('audio', { tone: 'error', title: 'Playback stopped.', text: `${state.audioError}. Reconnect your output device if needed, then retry.`, action: { button: 'retry_audio', label: 'Retry audio', enabled: state.audioRetry }, dismiss: 'hide' });
  if (state.saveError) add('save', { tone: 'error', text: state.saveError, action: { button: 'retry_save', label: 'Retry save', enabled: true }, dismiss: 'hide' });
  if (state.stemMixError) add('stem-mix', { tone: 'error', text: state.stemMixError, action: { button: 'retry_stem_mix', label: 'Retry save', enabled: state.stemMixRetry }, dismiss: 'hide' });
  if (state.error) add('error', { tone: 'error', text: state.error, dismiss: 'dismiss_error' });
  if (state.countInError) add('count-in', { tone: 'error', text: state.countInError, dismiss: 'hide' });
  if (state.clickError) add('click', { tone: 'error', text: state.clickError, dismiss: 'hide' });
  if (state.keymapWarning) add('keymap', { tone: 'error', text: state.keymapWarning, action: { button: 'reload_keymap', label: 'Reload configuration', enabled: !state.keymapLoading }, dismiss: 'hide' });
  if (state.processing) add('processing', {
    tone: 'progress', title: `${state.processing.label}${state.processing.progress === null ? '…' : ` · ${Math.round(state.processing.progress * 100)} %`}`,
    text: state.processing.detail,
    progress: state.processing.progress, action: { button: 'separation_open', label: 'Open Stems', enabled: true }
  });
  if (state.busy) add('busy', { tone: 'progress', text: state.busy, progress: null });
  if (state.undoCount && !state.busy) add(`undo-${state.undoCount}`, { tone: 'info', text: 'Section removed.', action: { button: 'undo_delete', label: 'Undo', enabled: true }, dismiss: 'hide' });
  if (state.stemWarning) add('stem-warning', { tone: 'info', text: `${state.stemWarning}${state.stemsUnavailable ? ' Using Original playback.' : ''}`, dismiss: 'hide' });
  if (state.info) add('info', { tone: 'info', text: state.info, dismiss: 'hide' });
  if (state.startupNotice) add('startup', { tone: 'info', text: state.startupNotice, dismiss: 'hide' });
  if (state.preview) add('preview', { tone: 'info', text: 'Interface preview · Audio playback and file opening are available in the desktop app.', dismiss: 'hide' });
  return list;
}
