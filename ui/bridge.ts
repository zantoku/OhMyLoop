import type {StemsSnapshot,StemTrack,StemProgress} from './stems';
import type {Catalogue,SongMetadata} from './library';
import {listen} from '@tauri-apps/api/event';
import { invoke, isTauri } from '@tauri-apps/api/core';
import type { Action, PracticeDocument, Snapshot, Song, Transport, LibraryStatus, ImportSelection, ImportProgress } from './types';
import type { KeymapFile } from './keymap';
import type { MapEdit, MeasureMap, MapPreview, MapView, AnalysisStatus, Candidates } from './timing';

export const native = isTauri();
const empty: Transport = { song_key: null, reference_token: null, position: 0, playing: false, finished: false, audio_error: null, save_error: null, device: null };

// The browser adapter exists for UI development and interaction testing only.
// It intentionally never pretends to play audio; the shipping app uses Rust.
let previewSong: Song | null = null;
let previewDocument: PracticeDocument | null = null;
let previewTransport = { ...empty };
const snapshot = (include = false): Snapshot => ({ song: include ? previewSong : null, document: structuredClone(previewDocument), transport: { ...previewTransport } });
function demo(): Snapshot {
  previewSong = { key: 'builtin-demo', reference_token:'builtin-demo', asset_id:'preview',timeline_id:'preview',reference_revision:1,session_id:'preview',  title: 'Practice sketch', path: null, duration: 12, rate: 48000,
    peaks: Array.from({ length: 4096 }, (_, i) => { const t = i / 4096 * 12; const level = 0.06 + 0.21 * Math.exp(-5 * (t % .5)); return [-level, level]; }) };
  const saved = localStorage.getItem('ohmyloop-ui-preview');
  previewDocument = saved ? JSON.parse(saved) : { version: 4, notes:'', song_key: 'builtin-demo',reference_token:'builtin-demo', controls: { speed: 1, semitones: 0 },
    sections: ['Verse', 'Chorus', 'Bridge', 'Outro'].map((name, i) => ({ id: `demo-${i}`, name, start: i * 3, end: (i + 1) * 3 })),
    selected_section: null, loop_selection: null, loop_enabled: false, last_position: 0 };
  previewDocument = {...previewDocument!,version:4,reference_token:'builtin-demo',notes:previewDocument!.notes??''};
  previewTransport = { ...empty, song_key: 'builtin-demo',reference_token:'builtin-demo', position: previewDocument!.last_position };
  return snapshot(true);
}

export const bridge = {
  stemsStatus:(referenceToken:string)=>native?invoke<StemsSnapshot>('stems_status',{referenceToken}):Promise.resolve({reference_token:referenceToken,set_id:null,tracks:[],available:false,warning:null,attenuation:null,model:null}),
  pickStemsDirectory:()=>native?invoke<string|null>('pick_stems_directory'):Promise.reject(new Error('Attach stems in the desktop app.')),
  attachStems:(referenceToken:string,path:string,operationId:string)=>native?invoke<StemsSnapshot>('attach_stems',{referenceToken,path,operationId}):Promise.reject(new Error('Attach stems in the desktop app.')),
  removeStems:(referenceToken:string,setId:string)=>native?invoke<StemsSnapshot>('remove_stems',{referenceToken,setId}):Promise.reject(new Error('Remove stems in the desktop app.')),
  setStemMix:(referenceToken:string,setId:string,tracks:StemTrack[],persist:boolean)=>native?invoke<StemsSnapshot>('set_stem_mix',{referenceToken,setId,tracks,persist}):Promise.reject(new Error('Mix stems in the desktop app.')),
  onStemProgress:(handler:(progress:StemProgress)=>void)=>native?listen<StemProgress>('stem-progress',event=>handler(event.payload)):Promise.resolve(()=>{}),
  libraryStatus:()=>native?invoke<LibraryStatus>('library_status'):Promise.resolve({selected:null,available:false,error:null,import_notice_seen:false}),
  chooseLibrary:(create:boolean,locate=false)=>native?invoke<Snapshot|null>('choose_library',{create,locate}):Promise.reject(new Error('Select libraries in the desktop app.')),
  retryLibrary:()=>native?invoke<Snapshot>('retry_library'):Promise.reject(new Error('Retry libraries in the desktop app.')),
  acknowledgeImportNotice:()=>native?invoke<void>('acknowledge_import_notice'):Promise.resolve(),
  pickImportSong:()=>native?invoke<ImportSelection|null>('pick_import_song'):Promise.reject(new Error('Import songs in the desktop app.')),
  importSong:(path:string)=>native?invoke<Snapshot>('import_song',{path}):Promise.reject(new Error('Import songs in the desktop app.')),
  deleteLibrarySong:(songKey:string)=>native?invoke<Snapshot>('delete_library_song',{songKey}):Promise.reject(new Error('Delete songs in the desktop app.')),
  onImportProgress:(handler:(progress:ImportProgress)=>void)=>native?listen<ImportProgress>('import-progress',event=>handler(event.payload)):Promise.resolve(()=>{}),
  onExitRequested:(handler:()=>void)=>native?listen('prepare-exit',handler):Promise.resolve(()=>{}),
  finishExit:()=>native?invoke<void>('finish_exit'):Promise.resolve(),
  readLibrary:()=>native?invoke<Catalogue>('read_library'):Promise.resolve({version:1,songs:[]}),
  openLibrarySong:(songKey:string)=>native?invoke<Snapshot>('open_library_song',{songKey}):Promise.reject(new Error('Open library songs in the desktop app.')),
  updateLibrarySong:(songKey:string,metadata:SongMetadata)=>native?invoke<Catalogue>('update_library_song',{songKey,metadata}):Promise.reject(new Error('Save library details in the desktop app.')),
  countIn:(referenceToken:string,enabled:boolean,repeat:boolean,volume:number)=>native?invoke<Transport>('set_count_in',{referenceToken,enabled,repeat,volume}):Promise.reject(new Error('Count-in is available in the desktop app.')),
  metronome:(referenceToken:string,enabled:boolean,volume:number,map:MeasureMap|null)=>native?invoke<Transport>('set_map_metronome',{referenceToken,enabled,volume,map}):Promise.reject(new Error('The map metronome is available in the desktop app.')),
  readCachedAnalysis: (referenceToken:string) => native ? invoke<Candidates|null>('read_cached_analysis',{referenceToken}) : Promise.resolve(null),
  analysisStatus: () => native ? invoke<AnalysisStatus>('analysis_status') : Promise.reject(new Error('Beat analysis is available in the desktop app.')),
  installModel: () => native ? invoke<AnalysisStatus>('install_analysis_model') : Promise.reject(new Error('Install the model in the desktop app.')),
  startAnalysis: (referenceToken:string, revision:number) => native ? invoke<AnalysisStatus>('start_analysis', {referenceToken,revision}) : Promise.reject(new Error('Beat analysis is available in the desktop app.')),
  cancelAnalysis: (id:number) => native ? invoke<AnalysisStatus>('cancel_analysis',{id}) : Promise.reject(new Error('Beat analysis is available in the desktop app.')),
  previewMap: (referenceToken: string, map: MeasureMap | null, edit: MapEdit) => native ? invoke<MapPreview>('preview_measure_map', { referenceToken, map, edit }) : Promise.reject(new Error('Musical map editing is available in the desktop app.')),
  mapView: (map: MeasureMap) => native ? invoke<MapView>('measure_map_view', { map }) : Promise.reject(new Error('Musical map editing is available in the desktop app.')),
  readKeymap: () => native ? invoke<KeymapFile>('read_keymap') : Promise.resolve({ path: null, configuration: null, warning: null }),
  initialize: () => native ? invoke<Snapshot>('initialize') : Promise.resolve(snapshot()),
  loadDemo: () => native ? invoke<Snapshot>('load_demo') : Promise.resolve(demo()),
  libraryReference:(songKey:string)=>native?invoke<string>('library_reference',{songKey}):Promise.reject(new Error('Replace recordings in the desktop app.')),
  replaceLibraryReference:(songKey:string,referenceToken:string,timingUnchanged:boolean)=>native?invoke<Snapshot|null>('replace_library_reference',{songKey,referenceToken,timingUnchanged}):Promise.reject(new Error('Replace recordings in the desktop app.')),
  replaceRecording:(referenceToken:string,timingUnchanged:boolean)=>native?invoke<Snapshot|null>('replace_recording',{referenceToken,timingUnchanged}):Promise.reject(new Error('Replace recordings in the desktop app.')),
  update: (document: PracticeDocument) => {
    if (native) return invoke<Snapshot>('update_document', { document });
    previewDocument = structuredClone(document); localStorage.setItem('ohmyloop-ui-preview', JSON.stringify(document));
    return Promise.resolve(snapshot());
  },
  action: (action: Action, referenceToken:string) => {
    if (native) return invoke<Snapshot>('transport_action', { action, referenceToken });
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
