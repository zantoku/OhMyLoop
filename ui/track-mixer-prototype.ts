// THROWAWAY fixture for the existing Practice screen. No files or persistence.
import type {Snapshot, Transport} from './types';
export function mixerPrototypeSnapshot(transport: Transport): Snapshot {
  return {song:{key:'prototype',reference_token:'prototype',asset_id:'prototype',timeline_id:'prototype',reference_revision:1,session_id:'prototype',title:'Bones — Amorphis',path:null,duration:296,rate:48000,peaks:Array.from({length:4096},(_,i)=>{const a=.08+.23*Math.abs(Math.sin(i*.043)*Math.cos(i*.013));return [-a,a];})},document:{version:4,song_key:'prototype',reference_token:'prototype',notes:'',controls:{speed:1,semitones:0},sections:[{id:'verse',name:'Verse',start:20,end:65},{id:'chorus',name:'Chorus',start:65,end:95},{id:'bridge',name:'Bridge',start:160,end:195}],selected_section:null,loop_selection:{start:65,end:95,fade_ms:10},loop_enabled:false,last_position:84},transport:{...transport,reference_token:'prototype',position:84}};
}
