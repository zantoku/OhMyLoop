<script lang="ts">
  import MeterFields from './MeterFields.svelte';
  import {meterForm,readMeter} from './meter-form';
  import {fractionText,parseFraction,pulseName,type MeasureMap,type MapView,type MeasureView,type MeasureKind,type MapEdit} from './timing';
  import {time} from './model';
  import type {Song} from './types';
  import type {Keymap} from './keymap';
  import type {ButtonId,MappingActionId} from './actions';
  let {song,map,mapView,measure,position,keymap,active,run,available,onedit}:
    {song:Song;map:MeasureMap|null;mapView:MapView|null;measure:MeasureView|null;position:number;keymap:Keymap;active:boolean;run:(id:ButtonId)=>void;available:(id:ButtonId)=>boolean;onedit:(value:MapEdit,preview?:boolean)=>Promise<void>}=$props();
  let panel=$state<'measure'|'beats'|'range'>('measure');
  let notation=$state(meterForm()),kind=$state<MeasureKind>('full'),extent=$state('4'),following=$state(false),origin=$state(1);
  let boundary=$state('start'),boundarySeconds=$state(0),beatId=$state(''),beatSeconds=$state(0),beatQuarter=$state('1');
  let first=$state(''),last=$state(''),error=$state(''),inspectorRevision='';
  const measures=$derived(mapView?.measures??[]);
  $effect(()=>{
    const m=measure,identity=JSON.stringify([m,mapView?.revision]);
    if(!m||identity===inspectorRevision)return;inspectorRevision=identity;
    notation=meterForm(m.meter);origin=map?.first_number??1;kind=m.kind;extent=fractionText(m.length);following=false;
    boundary='start';boundarySeconds=m.start/song.rate;beatId='';beatSeconds=(m.start+(m.end-m.start)/4)/song.rate;beatQuarter='1';first=m.id;last=m.id;error='';
  });
  function selectedId(){if(!measure)throw new Error('Select a measure first.');return measure.id;}
  export async function command(id:MappingActionId){
    error='';
    try{
      const commands:Partial<Record<MappingActionId,()=>void|Promise<void>>>={
        map_origin:()=>onedit({type:'set_origin',first_number:origin}),
        map_anchor_playhead:()=>{panel='beats';beatSeconds=position;},
        map_notation:()=>onedit({type:'set_notation',id:selectedId(),meter:readMeter(notation),kind,length:parseFraction(extent),following}),
        map_boundary:()=>onedit({type:'move_boundary',id:boundary==='start'?measure!.start_id:measure!.end_id,frame:Math.round(boundarySeconds*song.rate)},false),
        map_beat:()=>onedit({type:'set_beat',measure:selectedId(),id:beatId||null,frame:Math.round(beatSeconds*song.rate),offset:parseFraction(beatQuarter)},false),
        map_remove_beat:()=>onedit({type:'remove_beat',measure:selectedId(),id:beatId},false),
        map_split:()=>onedit({type:'split',id:selectedId(),frame:Math.round(beatSeconds*song.rate),left:parseFraction(beatQuarter)}),
        map_merge:()=>onedit({type:'merge',id:selectedId(),meter:readMeter(notation)}),
        map_remove:()=>onedit({type:'remove',id:selectedId()}),
        map_protect:()=>onedit({type:'set_protected',id:selectedId(),protected:!measure!.protected},false),
        map_respace:()=>onedit({type:'respace',first,last})
      };await commands[id]?.();
    }catch(e){error=String(e);}
  }
  function chooseBeat(){const beat=map?.measures.find(m=>m.id===measure?.id)?.beats.find(b=>b.id===beatId);if(beat){beatSeconds=beat.frame/song.rate;beatQuarter=fractionText(beat.offset);}}
  const title=(id:ButtonId)=>keymap.title(id),label=(id:ButtonId,fallback:string)=>keymap.buttonLabel(id,fallback),disabled=(id:ButtonId)=>!active||!available(id);
</script>
<section class="measure-tools" aria-label="Measure editing tools">
  <div class="tools-heading"><div><h3>{measure?`Measure ${measure.label}`:'Measure editing'}</h3>{#if measure}<span>{time(measure.start/song.rate,3)} – {time(measure.end/song.rate,3)} · {measure.tempo_bpm.toFixed(1)} BPM ({pulseName(measure.meter.pulse)}){measure.protected?' · Protected':''}</span>{/if}</div>
    <nav aria-label="Measure editing panels"><button aria-pressed={panel==='measure'} onclick={()=>{panel='measure';}}>Measure</button><button aria-pressed={panel==='beats'} onclick={()=>{panel='beats';}}>Beat anchors and splits</button><button aria-pressed={panel==='range'} onclick={()=>{panel='range';}}>Respace or free time</button></nav>
  </div>
  {#if error}<p class="tools-error" role="alert">{error}</p>{/if}
  {#if !measure}<p class="hint">Choose a measure in the ruler to edit its timing, meter, or extent.</p>
  {:else if panel==='measure'}
    <div class="measure-fields">
      <div class="notation-fields"><MeterFields bind:value={notation} layout="compact"/>
        <div class="field-row"><label>Measure kind<select bind:value={kind}><option value="full">Full measure</option><option value="pickup">Pickup</option><option value="incomplete">Incomplete measure</option></select></label>{#if kind!=='full'}<label>Actual duration (quarter notes)<input bind:value={extent} placeholder="1 or 3/2"/></label>{/if}
          <label class="check"><input type="checkbox" bind:checked={following}/>Apply meter until the next meter change</label>
          <button class="button" disabled={disabled('map_notation')} title={title('map_notation')} onclick={()=>run('map_notation')}>{label('map_notation','Preview notation')}</button>
        </div><p class="hint">Notation previews rebuild the internal beat grid.</p>
      </div>
      <div class="boundary-fields"><div class="field-row"><label>Boundary<select bind:value={boundary} onchange={()=>{boundarySeconds=(boundary==='start'?measure!.start:measure!.end)/song.rate;}}><option value="start">Start</option><option value="end">End</option></select></label><label>Position (seconds)<input type="number" min="0" max={song.duration} step="any" bind:value={boundarySeconds}/></label><button class="button" disabled={disabled('map_boundary')} title={title('map_boundary')} onclick={()=>run('map_boundary')}>{label('map_boundary','Move boundary')}</button></div>
        <div class="field-row"><label>First full measure number<input type="number" min="1" max="1000000" step="1" bind:value={origin}/></label><button class="text-button" disabled={!map||disabled('map_origin')} title={title('map_origin')} onclick={()=>run('map_origin')}>{label('map_origin','Preview numbering')}</button><button class="text-button" disabled={disabled('map_protect')} title={title('map_protect')} onclick={()=>run('map_protect')}>{label('map_protect',measure.protected?'Unprotect measure':'Protect measure')}</button></div>
      </div>
    </div>
  {:else if panel==='beats'}
    <div class="field-row"><label>Beat anchor<select bind:value={beatId} onchange={chooseBeat}><option value="">New anchor</option>{#each map?.measures.find(m=>m.id===measure?.id)?.beats??[] as b}<option value={b.id}>{fractionText(b.offset)} quarters · {time(b.frame/song.rate,3)}</option>{/each}</select></label><label>Anchor position (seconds)<input type="number" min="0" max={song.duration} step="any" bind:value={beatSeconds}/></label><label>Notated offset (quarter notes)<input bind:value={beatQuarter} placeholder="1 or 3/2"/></label><button class="text-button" disabled={disabled('map_anchor_playhead')} title={title('map_anchor_playhead')} onclick={()=>run('map_anchor_playhead')}>{label('map_anchor_playhead','Use playhead')}</button></div>
    <div class="tool-actions"><button class="button" disabled={disabled('map_beat')} title={title('map_beat')} onclick={()=>run('map_beat')}>{label('map_beat',beatId?'Update anchor':'Add anchor')}</button><button class="text-button" disabled={!beatId||disabled('map_remove_beat')} title={title('map_remove_beat')} onclick={()=>run('map_remove_beat')}>{label('map_remove_beat','Remove anchor')}</button><button class="button" disabled={disabled('map_split')} title={title('map_split')} onclick={()=>run('map_split')}>{label('map_split','Preview split')}</button><button class="text-button" disabled={disabled('map_merge')} title={title('map_merge')} onclick={()=>run('map_merge')}>{label('map_merge','Preview merge with next')}</button></div><p class="hint">Split at the source position and notated offset above.</p>
  {:else}
    <div class="field-row"><label>First measure<select bind:value={first}>{#each measures as m}<option value={m.id}>{m.label}</option>{/each}</select></label><label>Last measure<select bind:value={last}>{#each measures as m}<option value={m.id}>{m.label}</option>{/each}</select></label><button class="button" disabled={disabled('map_respace')} title={title('map_respace')} onclick={()=>run('map_respace')}>{label('map_respace','Preview spacing')}</button><button class="text-button" disabled={disabled('map_remove')} title={title('map_remove')} onclick={()=>run('map_remove')}>{label('map_remove','Make this measure free time')}</button></div><p class="hint">Keep outer boundaries fixed and rebuild spacing by notated duration. Internal anchors are removed.</p>
  {/if}
</section>
<style>
  .measure-tools{padding:13px 24px 15px;border-top:1px solid #e0e6f0;background:#f9fafc;min-width:0;max-height:230px;overflow:auto;grid-column:1/-1;color:#334966}
  .tools-heading{display:flex;align-items:center;justify-content:space-between;gap:12px;margin-bottom:9px}.tools-heading h3{font-size:13px;font-weight:600;display:inline;margin-right:10px}.tools-heading span{font-size:10px;color:#7a879d;font-variant-numeric:tabular-nums}.tools-heading nav{display:flex;gap:3px;flex-shrink:0}.tools-heading nav button{font:inherit;font-size:11px;border:0;padding:6px 8px;color:#667990;background:transparent;border-radius:4px}.tools-heading nav button[aria-pressed=true]{background:#e1e8f2;color:#334966}
  .measure-fields{display:grid;grid-template-columns:minmax(0,1.3fr) minmax(0,1fr);gap:24px}.field-row{display:flex;gap:10px;align-items:end;flex-wrap:wrap;margin-top:8px}.field-row label{flex:1;min-width:80px;max-width:220px}.field-row .check{flex:1.5;max-width:250px;flex-direction:row;align-items:center;line-height:1.4}.check input{width:auto;margin-right:3px}label{display:flex;flex-direction:column;gap:5px;font-size:10px;color:#596b86}input,select{min-width:0;width:100%;padding:6px 7px;border:1px solid #d3ddeb;border-radius:4px;background:#fff;color:#334966;font:inherit;font-size:12px}
  .button{font-size:11px;padding:7px 10px;white-space:nowrap}.text-button{font-size:11px;padding:7px 0}.tool-actions{display:flex;gap:14px;align-items:center;margin-top:12px;flex-wrap:wrap}.hint{font-size:10px;line-height:1.5;color:#7a879d;margin-top:8px}.tools-error{font-size:12px;color:#a7475c;background:#fff0f2;padding:8px 10px;border-radius:4px;margin-bottom:10px}
  @media(max-width:1000px){.measure-tools{padding:10px 17px;max-height:240px}.tools-heading{align-items:start;flex-wrap:wrap}.tools-heading span{display:block;margin-top:4px}.measure-fields{gap:16px;grid-template-columns:minmax(0,1.3fr) minmax(0,1fr)}}
  @media(max-height:740px){.measure-tools{max-height:190px;padding-block:9px}}
</style>
