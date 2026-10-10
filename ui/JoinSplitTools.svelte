<script lang="ts">
  import {fractionText,parseFraction,type MeasureMap,type MapView,type MeasureView,type MapEdit} from './timing';
  import {time} from './model';
  import type {Song} from './types';
  import type {Keymap} from './keymap';
  import type {ButtonId,MappingActionId} from './actions';
  let {song,map,mapView,measure,position,keymap,run,available,onedit,visible=true}:
    {song:Song;map:MeasureMap|null;mapView:MapView|null;measure:MeasureView;position:number;keymap:Keymap;run:(id:ButtonId)=>void;available:(id:ButtonId)=>boolean;onedit:(value:MapEdit,preview?:boolean)=>Promise<void>;visible?:boolean}=$props();
  let splitBeat=$state(2),beatId=$state(''),beatSeconds=$state(0),beatQuarter=$state('1'),advanced=$state(false),error=$state(''),identity='';
  const next=$derived(mapView?.measures[(mapView?.measures.findIndex(m=>m.id===measure.id)??-1)+1]??null);
  const splitPoints=$derived(measure.split_points);
  $effect(()=>{const value=JSON.stringify([measure,mapView?.revision]);if(value!==identity){identity=value;splitBeat=splitPoints[0]?.beat??2;beatId='';beatSeconds=(measure.start+(measure.end-measure.start)/4)/song.rate;beatQuarter='1';error='';}});
  export function canRun(id:MappingActionId){
    if(id==='map_anchor_playhead')return true;
    if(measure.protected)return false;
    if(id==='map_split')return splitPoints.some(p=>p.beat===splitBeat);
    if(id==='map_merge')return !!next&&!next.protected&&measure.end_id===next.start_id&&measure.meter.denominator===next.meter.denominator&&measure.meter.numerator+next.meter.numerator<=99;
    if(id==='map_remove_beat')return !!beatId;
    return true;
  }
  export async function command(id:MappingActionId){
    if(!canRun(id))return;error='';
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
{#if visible}
{#if error}<p class="error" role="alert">{error}</p>{/if}
<div class="operations">
  <div><button class="button" disabled={!available('map_merge')} title={title('map_merge')} onclick={()=>run('map_merge')}>{label('map_merge','Join with next')}</button>
    <p>{#if !next}There is no following measure to join.{:else if measure.end_id!==next.start_id}Free time separates these measures.{:else if measure.meter.denominator!==next.meter.denominator}Join requires the same denominator on both measures.{:else if measure.protected||next.protected}Unlock both measures before joining.{:else if measure.meter.numerator+next.meter.numerator>99}The combined numerator exceeds 99.{:else}Result: {measure.meter.numerator+next.meter.numerator}/{measure.meter.denominator}. Source timing is preserved.{/if}</p>
  </div>
  <div><div class="row"><label>Split before beat<select bind:value={splitBeat} disabled={!splitPoints.length}>{#each splitPoints as point}<option value={point.beat}>Beat {point.beat} · {point.anchor?'fixed':'interpolated'} · {time(point.frame/song.rate,3)}</option>{/each}</select></label><button class="button" disabled={!available('map_split')} title={title('map_split')} onclick={()=>run('map_split')}>{label('map_split','Split measure')}</button></div>
    <p>{#if splitPoints.length}Creates two partial {measure.meter.numerator}/{measure.meter.denominator} measures at the selected beat.{:else}There is no internal beat to split at.{/if}</p>
  </div>
</div>
<details bind:open={advanced}><summary>Advanced — beat anchor positions</summary>
  <div class="row"><label>Beat anchor<select bind:value={beatId} onchange={chooseBeat}><option value="">New anchor</option>{#each map?.measures.find(m=>m.id===measure.id)?.beats??[] as b}<option value={b.id}>{fractionText(b.offset)} quarters · {time(b.frame/song.rate,3)}</option>{/each}</select></label><label>Anchor position (seconds)<input type="number" min="0" max={song.duration} step="any" bind:value={beatSeconds}/></label><label>Notated offset (quarter notes)<input bind:value={beatQuarter} placeholder="1 or 3/2"/></label><button class="text-button" disabled={!available('map_anchor_playhead')} title={title('map_anchor_playhead')} onclick={()=>run('map_anchor_playhead')}>{label('map_anchor_playhead','Use playhead')}</button><button class="button" disabled={!available('map_beat')} title={title('map_beat')} onclick={()=>run('map_beat')}>{label('map_beat',beatId?'Update anchor':'Add anchor')}</button><button class="text-button" disabled={!available('map_remove_beat')} title={title('map_remove_beat')} onclick={()=>run('map_remove_beat')}>{label('map_remove_beat','Remove anchor')}</button></div>
</details>
{/if}
<style>
  .operations{display:grid;grid-template-columns:1fr 1.6fr;gap:24px;align-items:end}.row{display:flex;gap:10px;align-items:end;flex-wrap:wrap}label{display:flex;flex-direction:column;gap:5px;font-size:10px;color:#596b86;min-width:120px;max-width:260px;flex:1}input,select{width:100%;padding:6px 7px;border:1px solid #d3ddeb;border-radius:4px;background:#fff;color:#334966;font:inherit;font-size:12px}.button{font-size:11px;padding:7px 10px;white-space:nowrap}.text-button{font-size:11px;padding:7px 0}p{font-size:10px;line-height:1.5;color:#7a879d;margin-top:8px}details{margin-top:10px;border-top:1px solid #e0e6f0;padding-top:9px}summary{font-size:11px;color:#596b86;cursor:pointer;width:fit-content}details .row{margin-top:10px}.error{color:#a7475c;background:#fff0f2;padding:8px 10px}
</style>
