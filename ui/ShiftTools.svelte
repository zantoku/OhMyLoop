<script lang="ts">
  import {pulseName,type Candidates,type MapEdit,type MapView,type MeasureView} from './timing';
  import type {Keymap} from './keymap';
  import type {ButtonId,MappingActionId} from './actions';
  let {candidates,epsilonMs=$bindable(30),resnap=$bindable(true),mapView,measure,keymap,run,available,onedit,visible=true}:
    {candidates:Candidates|null;epsilonMs?:number;resnap?:boolean;mapView:MapView|null;measure:MeasureView;keymap:Keymap;run:(id:ButtonId)=>void;available:(id:ButtonId)=>boolean;onedit:(edit:MapEdit,preview?:boolean)=>Promise<void>;visible?:boolean}=$props();
  let pulses=$state(2);
  const useDetection=$derived(resnap&&!!candidates);
  const index=$derived(mapView?.measures.findIndex(m=>m.id===measure.id)??-1);
  const preceding=$derived(index>0?mapView!.measures[index-1]:null);
  const suffix=$derived(index>0?mapView!.measures.slice(index-1):[]);
  const gap=$derived(suffix.some((m,i)=>i>0&&suffix[i-1].end_id!==m.start_id));
  const protectedRange=$derived(suffix.some(m=>m.protected));
  export function canRun(){return !!preceding&&!gap&&!protectedRange&&Number.isInteger(pulses)&&pulses>=1&&pulses<=64&&(!resnap||(!!candidates&&Number.isFinite(epsilonMs)&&epsilonMs>=0&&epsilonMs<=1000));}
  export async function command(id:MappingActionId){if(id==='map_shift_resnap'){resnap=!resnap;return;}if(!canRun())return;await onedit({type:'shift',id:measure.id,pulses,direction:id==='map_shift_left'?'left':'right',...(useDetection?{resnap:{pulses:candidates!.beats.map(b=>b.frame),epsilon_ms:epsilonMs}}:{})},false);}
</script>
{#if visible}
<div class="shift"><div class="row"><label>Shift by (pulses)<input aria-label="Shift by pulses" type="number" min="1" max="64" step="1" bind:value={pulses}/></label><span>{pulseName(measure.meter.pulse)}-note pulses</span><button class="button" disabled={!available('map_shift_left')} title={keymap.title('map_shift_left')} onclick={()=>run('map_shift_left')}>{keymap.buttonLabel('map_shift_left','Shift left')}</button><button class="button" disabled={!available('map_shift_right')} title={keymap.title('map_shift_right')} onclick={()=>run('map_shift_right')}>{keymap.buttonLabel('map_shift_right','Shift right')}</button></div>
  <div class="refit"><label class="check"><input type="checkbox" checked={resnap} disabled={!available('map_shift_resnap')} title={keymap.title('map_shift_resnap')} onchange={()=>run('map_shift_resnap')}/>{keymap.buttonLabel('map_shift_resnap','Re-snap to detected beats')}</label>{#if resnap}<label>ε (ms)<input aria-label="Shift beat timing tolerance (ms)" type="number" min="0" max="1000" step="any" bind:value={epsilonMs}/></label>{/if}</div>
  {#if useDetection}<p>Rebuilds affected beat timing with the Create measures tolerance. Missing or extra detected pulses must be corrected first.</p>{:else if !candidates}<p>No detected beats available. Analyze the song, or turn re-snapping off to preserve authored timing.</p>{/if}
  <p>{#if !preceding}Select a measure with a preceding measure to shift.{:else if gap}Reclaim or map intervening free time before shifting the remaining measures.{:else if protectedRange}Unlock the affected measures before shifting.{:else}Moves this and following measures. Right extends the preceding meter; left extends the last measure. Consumed measures are absorbed. Endpoints stay fixed. Undo restores the map.{/if}</p>
</div>
{/if}
<style>
  .refit{display:flex;gap:12px;align-items:center;margin-top:10px}.check{flex-direction:row;align-items:center}.check input{width:auto}.shift{min-width:0}.row{display:flex;align-items:end;gap:10px;flex-wrap:wrap}label{display:flex;flex-direction:column;gap:5px;font-size:10px;color:#596b86}input{width:65px;padding:6px 7px;border:1px solid #d3ddeb;border-radius:4px;background:#fff;color:#334966;font:inherit;font-size:12px}.row>span{font-size:11px;color:#596b86;align-self:center}.button{font-size:11px;padding:7px 10px;white-space:nowrap}p{font-size:10px;line-height:1.5;color:#7a879d;margin-top:8px}
</style>
