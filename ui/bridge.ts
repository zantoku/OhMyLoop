import { invoke, isTauri } from '@tauri-apps/api/core';
import type { Action, PracticeDocument, Snapshot, Song, Transport } from './types';

export const native = isTauri();
const empty: Transport = { song_key: null, position: 0, playing: false, finished: false, audio_error: null, save_error: null, device: null };

// The browser adapter exists for UI development and interaction testing only.
// It intentionally never pretends to play audio; the shipping app uses Rust.
let previewSong: Song | null = null;
let previewDocument: PracticeDocument | null = null;
let previewTransport = { ...empty };
const snapshot = (include = false): Snapshot => ({ song: include ? previewSong : null, document: structuredClone(previewDocument), transport: { ...previewTransport } });
function demo(): Snapshot {
  previewSong = { key: 'builtin-demo', title: 'Practice sketch', path: null, duration: 12, rate: 48000,
    peaks: Array.from({ length: 4096 }, (_, i) => { const t = i / 4096 * 12; const level = 0.06 + 0.21 * Math.exp(-5 * (t % .5)); return [-level, level]; }) };
  const saved = localStorage.getItem('ohmyloop-ui-preview');
  previewDocument = saved ? JSON.parse(saved) : { version: 2, song_key: 'builtin-demo', controls: { speed: 1, semitones: 0 },
    sections: ['Verse', 'Chorus', 'Bridge', 'Outro'].map((name, i) => ({ id: `demo-${i}`, name, start: i * 3, end: (i + 1) * 3 })),
    selected_section: null, loop_selection: null, loop_enabled: false, last_position: 0 };
  previewTransport = { ...empty, song_key: 'builtin-demo', position: previewDocument!.last_position };
  return snapshot(true);
}

export const bridge = {
  initialize: () => native ? invoke<Snapshot>('initialize') : Promise.resolve(snapshot()),
  loadDemo: () => native ? invoke<Snapshot>('load_demo') : Promise.resolve(demo()),
  openSong: () => native ? invoke<Snapshot | null>('open_song') : Promise.reject(new Error('Open songs in the desktop app. This browser preview has no audio device.')),
  update: (document: PracticeDocument) => {
    if (native) return invoke<Snapshot>('update_document', { document });
    previewDocument = structuredClone(document); localStorage.setItem('ohmyloop-ui-preview', JSON.stringify(document));
    return Promise.resolve(snapshot());
  },
  action: (action: Action) => {
    if (native) return invoke<Snapshot>('transport_action', { action });
    if (action.type === 'seek') { previewTransport.position = action.seconds; if (previewDocument) previewDocument.last_position = action.seconds; }
    return Promise.resolve(snapshot());
  },
  status: () => native ? invoke<Transport>('transport_status') : Promise.resolve({ ...previewTransport })
};
