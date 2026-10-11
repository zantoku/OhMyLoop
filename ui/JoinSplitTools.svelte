<script lang="ts">
  import {fractionText,parseFraction,type Candidates,type MeasureMap,type MapView,type MeasureView,type MapEdit} from './timing';
  import {time} from './model';
  import ShiftTools from './ShiftTools.svelte';
  import type {Song} from './types';
  import type {Keymap} from './keymap';
  import type {ButtonId,MappingActionId} from './actions';
  let {song,candidates,epsilonMs=$bindable(30),resnap=$bindable(true),map,mapView,measure,position,keymap,run,available,onedit,visible=true}:
    {song:Song;candidates:Candidates|null;epsilonMs?:number;resnap?:boolean;map:MeasureMap|null;mapView:MapView|null;measure:MeasureView;position:number;keymap:Keymap;run:(id:ButtonId)=>void;available:(id:ButtonId)=>boolean;onedit:(value:MapEdit,preview?:boolean)=>Promise<void>;visible?:boolean}=$props();
  let splitBeat=$state(2),beatId=$state(''),beatSeconds=$state(0),beatQuarter=$state('1'),advanced=$state(false),error=$state(''),identity='';
  const next=$derived(mapView?.measures[(mapView?.measures.findIndex(m=>m.id===measure.id)??-1)+1]??null);
  const splitPoints=$derived(measure.split_points);
  let shift=$state<ShiftTools>();
  $effect(()=>{const value=JSON.stringify([measure,mapView?.revision]);if(value!==identity){identity=value;splitBeat=splitPoints[0]?.beat??2;beatId='';beatSeconds=(measure.start+(measure.end-measure.start)/4)/song.rate;beatQuarter='1';error='';}});
  export function canRun(id:MappingActionId){
    if(id==='map_shift_resnap')return true;
    if(id==='map_shift_left'||id==='map_shift_right')return shift?.canRun()??false;
    if(id==='map_anchor_playhead')return true;
    if(measure.protected)return false;
    if(id==='map_split')return splitPoints.some(p=>p.beat===splitBeat);
    if(id==='map_merge')return !!next&&!next.protected&&measure.end_id===next.start_id&&measure.meter.denominator===next.meter.denominator&&measure.meter.numerator+next.meter.numerator<=99;
    if(id==='map_remove_beat')return !!beatId;
    return true;
  }
  export async function command(id:MappingActionId){
    if(!canRun(id))return;error='';
    if(id==='map_shift_left'||id==='map_shift_right'||id==='map_shift_resnap'){await shift?.command(id);return;}
    try{
      if(id==='map_split')await onedit({type:'split_at_beat',id:measure.id,beat:splitBeat},false);
      if(id==='map_merge')await onedit({type:'merge',id:measure.id},false);
      if(id==='map_anchor_playhead'){advanced=true;beatSeconds=position;}
      if(id==='map_beat')await onedit({type:'set_beat',measure:measure.id,id:beatId||null,frame:Math.round(beatSeconds*song.rate),offset:parseFraction(beatQuarter)},false);
      if(id==='map_remove_beat')await onedit({type:'remove_beat',measure:measure.id,id:beatId},false);
    }catch(e){error=String(e);}
  }
  function chooseBeat(){const beat=map?.measures.find(m=>m.id===measure.id)?.beats.find(b=>b.id===beatId);if(beat){beatSeconds=beat.frame/song.rate;beatQuarter=fractionText(beat.offset);}}
  const title=(id:ButtonId)=>keymap.title(id),label=(id:ButtonId,fallback:string)=>keymap.buttonLabel(id,fallback);
</script>
{#if visible && error}<p class="error" role="alert">{error}</p>{/if}
<div class="operations" class:inactive={!visible}>
  {#if visible}
  <div class="join-tool"><div class="join-control"><button class="button" disabled={!available('map_merge')} title={title('map_merge')} onclick={()=>run('map_merge')}>{label('map_merge','Join with next')}</button></div>
    <p>{#if !next}There is no following measure to join.{:else if measure.end_id!==next.start_id}Free time separates these measures.{:else if measure.meter.denominator!==next.meter.denominator}Join requires the same denominator on both measures.{:else if measure.protected||next.protected}Unlock both measures before joining.{:else if measure.meter.numerator+next.meter.numerator>99}The combined numerator exceeds 99.{:else}Result: {measure.meter.numerator+next.meter.numerator}/{measure.meter.denominator}. Source timing is preserved.{/if}</p>
  </div>
  <div class="split-tool"><div class="row"><label class="split-beat">Split before beat<select bind:value={splitBeat} disabled={!splitPoints.length}>{#each splitPoints as point}<option value={point.beat}>{point.beat}</option>{/each}</select></label><button class="button" disabled={!available('map_split')} title={title('map_split')} onclick={()=>run('map_split')}>{label('map_split','Split measure')}</button></div>
    <p>{#if splitPoints.length}Creates two partial {measure.meter.numerator}/{measure.meter.denominator} measures at the selected beat.{:else}There is no internal beat to split at.{/if}</p>
  </div>
  {/if}
  <div class="shift-tool"><ShiftTools bind:resnap {candidates} bind:epsilonMs bind:this={shift} {mapView} {measure} {keymap} {run} {available} {onedit} {visible}/></div>
</div>
{#if visible}
<details bind:open={advanced}><summary>Advanced — beat anchor positions</summary>
  <div class="row"><label>Beat anchor<select bind:value={beatId} onchange={chooseBeat}><option value="">New anchor</option>{#each map?.measures.find(m=>m.id===measure.id)?.beats??[] as b}<option value={b.id}>{fractionText(b.offset)} quarters · {time(b.frame/song.rate,3)}</option>{/each}</select></label><label>Anchor position (seconds)<input type="number" min="0" max={song.duration} step="any" bind:value={beatSeconds}/></label><label>Notated offset (quarter notes)<input bind:value={beatQuarter} placeholder="1 or 3/2"/></label><button class="text-button" disabled={!available('map_anchor_playhead')} title={title('map_anchor_playhead')} onclick={()=>run('map_anchor_playhead')}>{label('map_anchor_playhead','Use playhead')}</button><button class="button" disabled={!available('map_beat')} title={title('map_beat')} onclick={()=>run('map_beat')}>{label('map_beat',beatId?'Update anchor':'Add anchor')}</button><button class="text-button" disabled={!available('map_remove_beat')} title={title('map_remove_beat')} onclick={()=>run('map_remove_beat')}>{label('map_remove_beat','Remove anchor')}</button></div>
</details>
{/if}
<style>
  .operations{display:grid;grid-template-columns:minmax(160px,.8fr) minmax(245px,1fr) minmax(330px,1.4fr);gap:20px;align-items:start}.operations.inactive{display:none}.split-tool,.shift-tool{border-left:1px solid var(--border);padding-left:20px;min-width:0}.join-control{padding-top:17px}.split-beat{flex:none;min-width:100px;width:100px}.split-beat select{width:76px;font-variant-numeric:tabular-nums}.row{display:flex;gap:10px;align-items:end;flex-wrap:wrap}label{display:flex;flex-direction:column;gap:5px;font-size:10px;color:var(--text-secondary);min-width:120px;max-width:260px;flex:1}input,select{width:100%;padding:6px 7px;border:1px solid var(--border-strong);border-radius:4px;background:var(--surface);color:var(--text);font:inherit;font-size:12px}.button{font-size:11px;padding:7px 10px;white-space:nowrap}.text-button{font-size:11px;padding:7px 0}p{font-size:10px;line-height:1.5;color:var(--muted);margin-top:8px}details{margin-top:10px;border-top:1px solid var(--border);padding-top:9px}summary{font-size:11px;color:var(--text-secondary);cursor:pointer;width:fit-content}details .row{margin-top:10px}.error{color:var(--error-text);background:var(--error-bg);padding:8px 10px}
  @media(max-width:900px){.operations{grid-template-columns:1fr;gap:12px}.split-tool,.shift-tool{border-left:0;border-top:1px solid var(--border);padding:12px 0 0}.join-control{padding-top:0}}
</style>
