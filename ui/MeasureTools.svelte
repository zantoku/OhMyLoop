<script lang="ts">
  import MeterFields from './MeterFields.svelte';
  import Icon from './Icon.svelte';
  import JoinSplitTools from './JoinSplitTools.svelte';
  import FreeTimeTools from './FreeTimeTools.svelte';
  import {meterForm,readMeter} from './meter-form';
  import {fractionText,parseFraction,pulseName,type Candidates,type MeasureMap,type MapView,type MeasureView,type MeasureKind,type MapEdit} from './timing';
  import {time} from './model';
  import type {Song} from './types';
  import type {Keymap} from './keymap';
  import type {ButtonId,MappingActionId} from './actions';
  let {song,candidates,epsilonMs=$bindable(30),resnap=$bindable(true),map,mapView,measure,position,keymap,active,run,available,onedit,onselect}:
    {song:Song;candidates:Candidates|null;epsilonMs?:number;resnap?:boolean;map:MeasureMap|null;mapView:MapView|null;measure:MeasureView|null;position:number;keymap:Keymap;active:boolean;run:(id:ButtonId)=>void;available:(id:ButtonId)=>boolean;onedit:(value:MapEdit,preview?:boolean)=>Promise<void>;onselect:(id:string|null)=>void}=$props();
  let panel=$state<'measure'|'join'|'free'>('measure');
  let join=$state<JoinSplitTools>(),free=$state<FreeTimeTools>();
  const joinActions:readonly MappingActionId[]=['map_split','map_merge','map_shift_left','map_shift_right','map_shift_resnap','map_beat','map_remove_beat','map_anchor_playhead'];
  export function canRun(id:MappingActionId){if(joinActions.includes(id))return join?.canRun(id)??false;if(id==='map_remove'||id==='map_reclaim')return free?.canRun(id)??false;return true;}
  export function selectFreeTime(frame:number){panel='free';free?.selectAt(frame);}
  let notation=$state(meterForm()),kind=$state<MeasureKind>('full'),extent=$state('4'),following=$state(false);
  let boundary=$state('start'),boundarySeconds=$state(0);
  let error=$state(''),inspectorRevision='';
  $effect(()=>{
    const m=measure,identity=JSON.stringify([m,mapView?.revision]);
    if(!m||identity===inspectorRevision)return;inspectorRevision=identity;
    notation=meterForm(m.meter);kind=m.kind;extent=fractionText(m.length);following=false;
    boundary='start';boundarySeconds=m.start/song.rate;error='';
  });
  function selectedId(){if(!measure)throw new Error('Select a measure first.');return measure.id;}
  export async function command(id:MappingActionId){
    error='';
    if(joinActions.includes(id)){panel='join';await join?.command(id);return;}
    if(id==='map_remove'||id==='map_reclaim'){panel='free';await free?.command(id);return;}
    try{
      const commands:Partial<Record<MappingActionId,()=>void|Promise<void>>>={
        map_notation:()=>onedit({type:'set_notation',id:selectedId(),meter:readMeter(notation),kind,length:parseFraction(extent),following},false),
        map_boundary:()=>onedit({type:'move_boundary',id:boundary==='start'?measure!.start_id:measure!.end_id,frame:Math.round(boundarySeconds*song.rate)},false),
        map_protect:()=>onedit({type:'set_protected',id:selectedId(),protected:!measure!.protected},false)
      };await commands[id]?.();
    }catch(e){error=String(e);}
  }
  const title=(id:ButtonId)=>keymap.title(id),label=(id:ButtonId,fallback:string)=>keymap.buttonLabel(id,fallback),disabled=(id:ButtonId)=>!active||!available(id);
</script>
<section class="measure-tools" aria-label="Measure editing tools">
  <div class="tools-heading"><div><div class="measure-title"><h3>{measure?`Measure ${measure.label}`:'Free time'}</h3>{#if measure}<button class="measure-lock" class:locked={measure.protected} aria-label={label('map_protect',measure.protected?'Unlock measure':'Lock measure')} aria-pressed={measure.protected} disabled={disabled('map_protect')} title={title('map_protect')} onclick={()=>run('map_protect')}><Icon name={measure.protected?'lock':'unlock'} size={16}/></button>{/if}</div>{#if measure}<span>{time(measure.start/song.rate,3)} – {time(measure.end/song.rate,3)} · {measure.tempo_bpm.toFixed(1)} BPM ({pulseName(measure.meter.pulse)})</span>{/if}</div>
    <nav aria-label="Measure editing panels"><button aria-pressed={panel==='measure'&&!!measure} disabled={!measure} onclick={()=>{panel='measure';}}>Measure</button><button aria-pressed={panel==='join'&&!!measure} disabled={!measure} onclick={()=>{panel='join';}}>Join / Split / Shift</button><button aria-pressed={panel==='free'||!measure} onclick={()=>{panel='free';}}>Free time</button></nav>
  </div>
  {#if error}<p class="tools-error" role="alert">{error}</p>{/if}
  {#if measure&&panel==='measure'}
    <div class="measure-row"><MeterFields bind:value={notation} layout="compact" showPulse={false} showGrouping={false}/>
      <label>Measure kind<select bind:value={kind}><option value="full">Full measure</option><option value="pickup">Pickup</option><option value="incomplete">Incomplete measure</option></select></label>{#if kind!=='full'}<label>Actual duration (quarter notes)<input bind:value={extent} placeholder="1 or 3/2"/></label>{/if}
      <button class="button" disabled={disabled('map_notation')} title={title('map_notation')} onclick={()=>run('map_notation')}>{label('map_notation','Apply changes')}</button>
      <span class="apply-hint">Changes rebuild beat spacing. Undo restores the previous map.</span>
    </div>
    <details class="measure-advanced"><summary>Advanced</summary>
      <div class="advanced-fields"><div><MeterFields bind:value={notation} layout="compact" showMeter={false}/>
        <p class="hint">Pulse is the click interval: 6/8 often uses a dotted quarter. Grouping counts denominator notes, e.g. 2+2+3 in 7/8.</p>
        <label class="check"><input type="checkbox" bind:checked={following}/>Apply meter until the next meter change</label>
      </div><div><div class="field-row"><label>Boundary<select bind:value={boundary} onchange={()=>{boundarySeconds=(boundary==='start'?measure!.start:measure!.end)/song.rate;}}><option value="start">Start</option><option value="end">End</option></select></label><label>Position (seconds)<input type="number" min="0" max={song.duration} step="any" bind:value={boundarySeconds}/></label><button class="button" disabled={disabled('map_boundary')} title={title('map_boundary')} onclick={()=>run('map_boundary')}>{label('map_boundary','Move boundary')}</button></div>
        <p class="hint">Moving a boundary evenly spaces beats in both adjacent measures. Undo restores their anchors.</p>
      </div></div>
    </details>
  {/if}
  {#if measure}<JoinSplitTools bind:resnap {candidates} bind:epsilonMs bind:this={join} {song} {map} {mapView} {measure} {position} {keymap} {run} {available} {onedit} visible={panel==='join'}/>{/if}
  <FreeTimeTools bind:this={free} {song} {mapView} {measure} {position} {keymap} {run} {available} {onedit} visible={!measure||panel==='free'} onuncounted={()=>onselect(null)} onreclaim={id=>{onselect(id);panel='measure';}}/>
</section>
<style>
  .measure-tools{padding:13px 24px 15px;border-top:1px solid #e0e6f0;background:#f9fafc;min-width:0;max-height:230px;overflow:auto;grid-column:1/-1;color:#334966}
  .tools-heading{display:flex;align-items:center;justify-content:space-between;gap:12px;margin-bottom:9px}.tools-heading h3{font-size:13px;font-weight:600}.measure-title{display:flex;gap:7px;align-items:center;margin-bottom:3px}.measure-lock{display:grid;place-items:center;width:25px;height:25px;padding:0;border:1px solid transparent;border-radius:4px;background:transparent;color:#8190a7}.measure-lock:hover{background:#e1e8f2;color:#334966}.measure-lock.locked{color:#4a7162;background:#e1ede7;border-color:#c7ddd2}.measure-lock:disabled{opacity:.45}.measure-lock:focus-visible{outline:2px solid #6d82bb;outline-offset:2px}.tools-heading span{font-size:10px;color:#7a879d;font-variant-numeric:tabular-nums}.tools-heading nav{display:flex;gap:3px;flex-shrink:0}.tools-heading nav button{font:inherit;font-size:11px;border:0;padding:6px 8px;color:#667990;background:transparent;border-radius:4px}.tools-heading nav button[aria-pressed=true]{background:#e1e8f2;color:#334966}
  .measure-row{display:flex;gap:14px;align-items:end;flex-wrap:wrap}.measure-row>label{width:160px}.measure-row input,.measure-row select{height:36px}.apply-hint{font-size:10px;color:#7a879d;align-self:center;max-width:210px;line-height:1.5}.measure-advanced{margin-top:10px;border-top:1px solid #e0e6f0;padding-top:9px}.measure-advanced summary{font-size:11px;color:#596b86;cursor:pointer;width:fit-content}.advanced-fields{display:grid;grid-template-columns:minmax(0,1fr) minmax(0,1fr);gap:24px;margin-top:10px}.field-row{display:flex;gap:10px;align-items:end;flex-wrap:wrap;margin-top:8px}.field-row label{flex:1;min-width:80px;max-width:220px}.check{display:flex;flex-direction:row;align-items:center;gap:5px;line-height:1.4;margin-top:8px}.check input{width:auto;margin-right:3px}label{display:flex;flex-direction:column;gap:5px;font-size:10px;color:#596b86}input,select{min-width:0;width:100%;padding:6px 7px;border:1px solid #d3ddeb;border-radius:4px;background:#fff;color:#334966;font:inherit;font-size:12px}
  .button{font-size:11px;padding:7px 10px;white-space:nowrap}.hint{font-size:10px;line-height:1.5;color:#7a879d;margin-top:8px}.tools-error{font-size:12px;color:#a7475c;background:#fff0f2;padding:8px 10px;border-radius:4px;margin-bottom:10px}
  @media(max-width:1000px){.measure-tools{padding:10px 17px;max-height:240px}.tools-heading{align-items:start;flex-wrap:wrap}.tools-heading span{display:block;margin-top:4px}.advanced-fields{gap:16px}}
  @media(max-height:740px){.measure-tools{max-height:190px;padding-block:9px}}
</style>
