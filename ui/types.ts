export type Controls = { speed: number; semitones: number };
export type Loop = { start: number; end: number; fade_ms: number };
export type Section = { id: string; name: string; start: number; end: number };
export type PracticeDocument = {
  version: number; song_key: string; reference_token: string; controls: Controls; sections: Section[];
  selected_section: string | null; loop_selection: Loop | null; loop_enabled: boolean; last_position: number; notes?:string;
  measure_map?: import('./timing').MeasureMap | null;
};
export type Song = { key: string; reference_token: string; asset_id: string; timeline_id: string; reference_revision: number; session_id: string; title: string; path: string | null; duration: number; rate: number; peaks: [number, number][] };
export type Transport = { song_key: string | null; reference_token: string | null; position: number; playing: boolean; finished: boolean; counting_in?:boolean; audio_error: string | null; save_error: string | null; device: string | null; musical_position?: import('./timing').MusicalPosition | null };
export type Snapshot = { song: Song | null; document: PracticeDocument | null; transport: Transport; library_status?:LibraryStatus; library_error?:string|null; startup_notice?:string|null; map_view?: import('./timing').MapView | null };
export type Action = { type: 'toggle_play' | 'pause' | 'retry_audio' } | { type: 'seek'; seconds: number };
export type View = { start: number; end: number };

export type LibraryStatus = { selected: { id: string; root: string } | null; available: boolean; error: string | null; import_notice_seen: boolean };
export type ImportSelection = { path: string; candidates: import('./library').LibraryEntry[] };
export type ImportProgress = { copied: number; total: number };
