<script lang="ts">
  import type {MeasureMap,MapEdit} from './timing';
  import {readGrouping} from './meter-form';
  import type {Keymap} from './keymap';
  import type {ButtonId,MappingActionId} from './actions';
  let {songKey,map,editable,keymap,run,available,onedit}:
    {songKey:string;map:MeasureMap|null;editable:boolean;keymap:Keymap;run:(id:ButtonId)=>void;available:(id:ButtonId)=>boolean;onedit:(edit:MapEdit)=>Promise<void>}=$props();
  let origin=$state(1),identity='';
  let signature=$state(''),grouping=$state(''),previousSong='';
  const signatures=$derived([...new Map((map?.measures??[]).map(m=>[`${m.meter.numerator}/${m.meter.denominator}`,m.meter])).entries()]);
  const meter=$derived(signatures.find(([key])=>key===signature)?.[1]);
  $effect(()=>{
    const next=JSON.stringify([songKey,map?.revision]);
    if(next===identity)return;
    identity=next;origin=map?.first_number??1;
    if(songKey!==previousSong||!signatures.some(([key])=>key===signature))signature=signatures[0]?.[0]??'';
    previousSong=songKey;
    grouping=map?.measures.find(m=>`${m.meter.numerator}/${m.meter.denominator}`===signature)?.meter.grouping.join('+')??'';
  });
  function selectSignature(){grouping=map?.measures.find(m=>`${m.meter.numerator}/${m.meter.denominator}`===signature)?.meter.grouping.join('+')??'';}
  const counts=$derived.by(()=>{
    let values:number[]|null=null;
    try{values=readGrouping(grouping);}catch{/* Invalid input is reported when previewing. */}
    const matches=map?.measures.filter(m=>m.meter.numerator===meter?.numerator&&m.meter.denominator===meter?.denominator)??[];
    return {changed:matches.filter(m=>!m.protected&&(!values||m.meter.grouping.join('+')!==values.join('+'))).length,locked:matches.filter(m=>m.protected).length};
  });
  export function canRun(id:MappingActionId){return id!=='map_grouping'||!!meter&&counts.changed>0;}
  export async function command(id:MappingActionId){
    if(id==='map_origin')await onedit({type:'set_origin',first_number:origin});
    if(id==='map_grouping'&&meter)await onedit({type:'set_grouping_for_meter',numerator:meter.numerator,denominator:meter.denominator,grouping:readGrouping(grouping)});
  }
</script>
<details open class="batch-tools"><summary>Batch tools</summary>
  <label>First full measure number<input type="number" min="1" max="1000000" step="1" bind:value={origin} disabled={!editable||!map?.measures.length}/></label>
  <button class="text-button" disabled={!available('map_origin')} title={keymap.title('map_origin')} onclick={()=>run('map_origin')}>{keymap.buttonLabel('map_origin','Preview numbering')}</button>
  <div class="grouping-form">
    <label>Time signature<select bind:value={signature} onchange={selectSignature} disabled={!editable||!signatures.length}>{#each signatures as [key]}<option value={key}>{key}</option>{/each}</select></label>
    <label>Beat grouping (optional)<input aria-label="Batch beat grouping (optional)" bind:value={grouping} placeholder="2+2+3" disabled={!editable||!meter}/></label>
    <p>Grouping counts denominator notes and must add up to the numerator. Leave blank to clear grouping.</p>
    <p>Replaces grouping throughout the song, including pickups and incomplete measures. Pulse controls click spacing; group starts accent clicks only when they coincide with a pulse.</p>
    {#if meter}<p role="status">{counts.changed} {counts.changed===1?'measure':'measures'} will change; {counts.locked} locked {counts.locked===1?'measure':'measures'} skipped.</p>{:else}<p>Create measures to use batch grouping.</p>{/if}
    <button class="text-button" disabled={!available('map_grouping')} title={keymap.title('map_grouping')} onclick={()=>run('map_grouping')}>{keymap.buttonLabel('map_grouping','Preview grouping')}</button>
  </div>
</details>
<style>
  .batch-tools{padding:9px 0 17px;border-bottom:1px solid var(--border)}summary{font-size:13px;font-weight:600;cursor:pointer;color:var(--text)}label{display:flex;flex-direction:column;gap:5px;font-size:11px;margin-top:11px;color:var(--text-secondary)}input,select{min-width:0;width:100%;padding:7px 8px;border:1px solid var(--border-strong);border-radius:4px;background:var(--surface);color:var(--text);font:inherit;font-size:12px}.text-button{font-size:11px;padding:5px 0;margin-top:4px}p{color:var(--muted);line-height:1.5;font-size:11px;margin:9px 0}.grouping-form{border-top:1px solid var(--border);margin-top:10px;padding-top:2px}
</style>
