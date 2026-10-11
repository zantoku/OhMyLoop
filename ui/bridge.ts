import type {Catalogue,SongMetadata} from './library';
import {listen} from '@tauri-apps/api/event';
import { invoke, isTauri } from '@tauri-apps/api/core';
import type { Action, PracticeDocument, Snapshot, Song, Transport } from './types';
import type { KeymapFile } from './keymap';
import type { MapEdit, MeasureMap, MapPreview, MapView, AnalysisStatus, Candidates } from './timing';

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
  previewDocument = saved ? JSON.parse(saved) : { version: 4, notes:'', song_key: 'builtin-demo', controls: { speed: 1, semitones: 0 },
    sections: ['Verse', 'Chorus', 'Bridge', 'Outro'].map((name, i) => ({ id: `demo-${i}`, name, start: i * 3, end: (i + 1) * 3 })),
    selected_section: null, loop_selection: null, loop_enabled: false, last_position: 0 };
  previewDocument = {...previewDocument!,version:4,notes:previewDocument!.notes??''};
  previewTransport = { ...empty, song_key: 'builtin-demo', position: previewDocument!.last_position };
  return snapshot(true);
}

export const bridge = {
  onExitRequested:(handler:()=>void)=>native?listen('prepare-exit',handler):Promise.resolve(()=>{}),
  finishExit:()=>native?invoke<void>('finish_exit'):Promise.resolve(),
  readLibrary:()=>native?invoke<Catalogue>('read_library'):Promise.resolve({version:1,songs:[]}),
  openLibrarySong:(songKey:string)=>native?invoke<Snapshot>('open_library_song',{songKey}):Promise.reject(new Error('Open library songs in the desktop app.')),
  updateLibrarySong:(songKey:string,metadata:SongMetadata)=>native?invoke<Catalogue>('update_library_song',{songKey,metadata}):Promise.reject(new Error('Save library details in the desktop app.')),
  metronome:(songKey:string,enabled:boolean,volume:number,map:MeasureMap|null)=>native?invoke<Transport>('set_map_metronome',{songKey,enabled,volume,map}):Promise.reject(new Error('The map metronome is available in the desktop app.')),
  readCachedAnalysis: (songKey:string) => native ? invoke<Candidates|null>('read_cached_analysis',{songKey}) : Promise.resolve(null),
  analysisStatus: () => native ? invoke<AnalysisStatus>('analysis_status') : Promise.reject(new Error('Beat analysis is available in the desktop app.')),
  installModel: () => native ? invoke<AnalysisStatus>('install_analysis_model') : Promise.reject(new Error('Install the model in the desktop app.')),
  startAnalysis: (songKey:string, revision:number) => native ? invoke<AnalysisStatus>('start_analysis', {songKey,revision}) : Promise.reject(new Error('Beat analysis is available in the desktop app.')),
  cancelAnalysis: (id:number) => native ? invoke<AnalysisStatus>('cancel_analysis',{id}) : Promise.reject(new Error('Beat analysis is available in the desktop app.')),
  previewMap: (songKey: string, map: MeasureMap | null, edit: MapEdit) => native ? invoke<MapPreview>('preview_measure_map', { songKey, map, edit }) : Promise.reject(new Error('Musical map editing is available in the desktop app.')),
  mapView: (map: MeasureMap) => native ? invoke<MapView>('measure_map_view', { map }) : Promise.reject(new Error('Musical map editing is available in the desktop app.')),
  readKeymap: () => native ? invoke<KeymapFile>('read_keymap') : Promise.resolve({ path: null, configuration: null, warning: null }),
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
    if (action.type === 'seek') {
      previewTransport.position = action.seconds;
      if (previewDocument) {
        previewDocument.last_position = action.seconds;
        const loop = previewDocument.loop_selection;
        if (previewDocument.loop_enabled && loop && (action.seconds < loop.start || action.seconds >= loop.end)) previewDocument.loop_enabled = false;
      }
    }
    return Promise.resolve(snapshot());
  },
  status: () => native ? invoke<Transport>('transport_status') : Promise.resolve({ ...previewTransport })
};
