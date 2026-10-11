<script lang="ts">
  import {tick} from 'svelte';
  import MeterFields from './MeterFields.svelte';
  import {meterForm,readMeter} from './meter-form';
  import {unmappedRanges} from './map-ranges';
  import type {MapView,MeasureView,MapEdit} from './timing';
  import type {Song} from './types';
  import type {Keymap} from './keymap';
  import type {ButtonId,MappingActionId} from './actions';
  import {time} from './model';
  let {song,mapView,measure,position,keymap,run,available,onedit,onuncounted,onreclaim,visible=true}:
    {song:Song;mapView:MapView|null;measure:MeasureView|null;position:number;keymap:Keymap;run:(id:ButtonId)=>void;available:(id:ButtonId)=>boolean;onedit:(value:MapEdit,preview?:boolean)=>Promise<void>;onuncounted:()=>void;onreclaim:(id:string)=>void;visible?:boolean}=$props();
  let rangeKey=$state(''),wanted=$state<number|null>(null),notation=$state(meterForm()),error=$state('');
  const ranges=$derived(unmappedRanges(mapView?.measures??[],Math.round(song.duration*song.rate)));
  const key=(range:{start:number;end:number})=>`${range.start}:${range.end}`;
  const chosen=$derived(ranges.find(r=>key(r)===rangeKey)??null);
  $effect(()=>{
    const current=ranges,desired=wanted;
    const requested=desired===null?null:current.find(r=>r.start<=desired&&desired<r.end);
    if(requested){rangeKey=key(requested);wanted=null;}
    else if(!current.some(r=>key(r)===rangeKey)){const frame=Math.round(position*song.rate),range=current.find(r=>r.start<=frame&&frame<r.end)??current[0];rangeKey=range?key(range):'';}
  });
  export function selectAt(frame:number){wanted=frame;const range=ranges.find(r=>r.start<=frame&&frame<r.end);rangeKey=range?key(range):'';}
  export function canRun(id:MappingActionId){return id==='map_reclaim'?!!chosen:id==='map_remove'?!!measure&&!measure.protected:false;}
  export async function command(id:MappingActionId){
    if(!canRun(id))return;error='';
    try{
      if(id==='map_remove'&&measure){const target=measure;wanted=target.start;notation=meterForm({...target.meter,grouping:[]});await onedit({type:'remove',id:target.id},false);await tick();if(!mapView?.measures.some(m=>m.id===target.id))onuncounted();}
      if(id==='map_reclaim'&&chosen){const target=chosen;await onedit({type:'reclaim',start:target.start,end:target.end,meter:readMeter(notation)},false);await tick();const created=mapView?.measures.find(m=>m.start===target.start&&m.end===target.end);if(created)onreclaim(created.id);}
    }catch(e){error=String(e);}
  }
  const label=(id:ButtonId,fallback:string)=>keymap.buttonLabel(id,fallback);
</script>
{#if visible}
{#if error}<p class="error" role="alert">{error}</p>{/if}
{#if measure}<div class="remove"><button class="text-button" disabled={!available('map_remove')} title={keymap.title('map_remove')} onclick={()=>run('map_remove')}>{label('map_remove',`Mark measure ${measure.label} as free time`)}</button><span>Keep the audio, remove its measure map. Undo restores it.</span></div>{/if}
{#if ranges.length}<div class="restore"><label>Uncounted range<select bind:value={rangeKey} onchange={()=>{wanted=null;}}>{#each ranges as range}<option value={key(range)}>{time(range.start/song.rate,3)} – {time(range.end/song.rate,3)}</option>{/each}</select></label><MeterFields bind:value={notation} prefix="Reclaim " layout="compact" showGrouping={false}/><button class="button" disabled={!available('map_reclaim')} title={keymap.title('map_reclaim')} onclick={()=>run('map_reclaim')}>{label('map_reclaim','Create measure from range')}</button></div><p>Creates one full measure over the selected range. Its beats are evenly spaced; existing measures stay in place.</p>
{:else}<p>No uncounted ranges. Mark a measure as free time to leave its audio uncounted.</p>{/if}
{/if}
<style>
  .remove{display:flex;align-items:center;gap:14px;margin-bottom:10px}.remove span,p{font-size:10px;line-height:1.5;color:var(--muted)}p{margin-top:8px}.restore{display:flex;gap:14px;align-items:end;flex-wrap:wrap}label{display:flex;flex-direction:column;gap:5px;font-size:10px;color:var(--text-secondary);min-width:180px}select{padding:6px 7px;border:1px solid var(--border-strong);border-radius:4px;background:var(--surface);color:var(--text);font:inherit;font-size:12px}.button{font-size:11px;padding:7px 10px;white-space:nowrap}.text-button{font-size:11px;padding:7px 0}.error{color:var(--error-text);background:var(--error-bg);padding:8px 10px}
</style>
