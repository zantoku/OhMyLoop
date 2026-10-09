<script lang="ts">
  import { untrack, onDestroy } from 'svelte';
  import { AnalysisSession, type AnalysisState } from './analysis-session';
  import MapTimeline from './MapTimeline.svelte';
  import Icon from './Icon.svelte';
  import { MappingSession, type MappingState } from './mapping-session';
  import { commonMeter, fractionText, parseFraction, type MapEdit, type MapPreview, type MeasureMap, type MapView, type MeasureKind, type Meter } from './timing';
  import type { Song, Transport, View } from './types';
  import type { Keymap } from './keymap';
  import type { ButtonId, MappingActionId } from './actions';
  import { time } from './model';
  let { song, map, mapView, transport, keymap, active, oncommit, onseek, run, available }:
    { song: Song; map: MeasureMap | null; mapView: MapView | null; transport: Transport; keymap: Keymap; active: boolean;
      oncommit: (value: MapPreview, songKey:string) => Promise<void>; onseek: (seconds:number)=>void; run:(id:ButtonId)=>void; available:(id:ButtonId)=>boolean } = $props();
  let editState = $state<MappingState>({pending:false,error:'',preview:null,undo:0,redo:0});
  let analysis = $state<AnalysisState>({status:null,requesting:false,error:''});
  const analyzer = new AnalysisSession(()=>({key:song.key,revision:map?.revision??0}),value=>{analysis=value;});
  const candidates = $derived(analysis.status?.result?.song_key===song.key?analysis.status.result:null);
  let hintedAnalysis=0;
  $effect(()=>{
    if(!active)return;
    void analyzer.refresh();const timer=setInterval(()=>{void analyzer.refresh();},400);
    return()=>clearInterval(timer);
  });
  onDestroy(()=>{analyzer.dispose();session.dispose();});
  const session = new MappingSession(()=>({key:song.key,map:map?$state.snapshot(map):null}),value=>oncommit(value,song.key),next=>{editState=next;});
  let view = $state<View>(untrack(()=>({start:0,end:song.duration})));
  let selected = $state<string|null>(untrack(()=>mapView?.measures[0]?.id??null));
  const displayed = $derived(editState.preview?.view??mapView);
  const measures = $derived(displayed?.measures??[]);
  const measure = $derived(measures.find(m=>m.id===selected)??null);
  let buildStart=$state(0),buildEnd=$state(untrack(()=>song.duration)),count=$state(untrack(()=>Math.max(1,Math.round(song.duration/2))));
  let numerator=$state(4),denominator=$state(4),pulse=$state('1'),grouping=$state('');
  let kind=$state<MeasureKind>('full'),extent=$state('4'),following=$state(false);
  let boundary=$state('start'),boundarySeconds=$state(0),beatId=$state(''),beatSeconds=$state(0),beatQuarter=$state('1');
  let first=$state(''),last=$state(''),formError=$state('');
  let inspectorRevision='';
  $effect(()=>{if(!measures.some(m=>m.id===selected))selected=measures[0]?.id??null;});
  $effect(()=>{
    const m=measure;const identity=`${m?.id}:${displayed?.revision}`;
    if(!m||identity===inspectorRevision)return;inspectorRevision=identity;
    numerator=m.meter.numerator;denominator=m.meter.denominator;pulse=fractionText(m.meter.pulse);grouping=m.meter.grouping.join('+');
    kind=m.kind;extent=fractionText(m.length);following=false;boundary='start';boundarySeconds=m.start/song.rate;
    beatId='';beatSeconds=(m.start+(m.end-m.start)/4)/song.rate;beatQuarter='1';first=m.id;last=m.id;
  });
  $effect(()=>{
    if(!candidates||hintedAnalysis===analysis.status?.id)return;
    hintedAnalysis=analysis.status!.id;
    if(!map?.measures.length){const first=candidates.beats.find(b=>b.downbeat)??candidates.beats[0];if(first)buildStart=first.frame/song.rate;}
  });
  function meter():Meter{
    const groups=grouping.trim()?grouping.split('+').map(v=>Number(v.trim())):[];
    if(groups.some(n=>!Number.isInteger(n)))throw new Error('Enter grouping as whole numbers separated by +, such as 2+2+3.');
    return {...commonMeter(),numerator,denominator,pulse:parseFraction(pulse),grouping:groups};
  }
  async function edit(value:MapEdit,preview=true){formError='';await session.edit(value,preview);}
  function selectedId(){if(!measure)throw new Error('Select a measure first.');return measure.id;}
  export function canRun(id:MappingActionId):boolean{
    if(editState.pending)return false;
    if(id==='map_apply'||id==='map_discard')return !!editState.preview;
    if(id==='map_undo')return editState.undo>0;if(id==='map_redo')return editState.redo>0;
    if(['map_notation','map_boundary','map_beat','map_remove_beat','map_split','map_merge','map_remove','map_protect','map_respace','map_fit'].includes(id))return !!measure&&!editState.preview;
    if(id==='map_analyze')return !!analysis.status?.model_ready&&!analysis.status.running&&!analysis.requesting&&!editState.preview;
    if(id==='map_cancel_analysis')return !!analysis.status?.running&&!analysis.requesting;
    if(id==='map_install_model')return !analysis.status?.running&&!analysis.requesting;
    if(id==='map_use_analysis')return !!candidates&&!analysis.status?.running&&!editState.preview;
    return true;
  }
  export async function command(id:MappingActionId){
    if(!canRun(id))return;
    try{
      const commands:Partial<Record<MappingActionId,()=>void|Promise<void>>>={
        map_analyze:()=>analyzer.start(),map_cancel_analysis:()=>analyzer.cancel(),map_install_model:()=>analyzer.start(true),
        map_use_analysis:()=>edit({type:'build_from_pulses',pulses:candidates!.beats.map(b=>b.frame),start:Math.round(buildStart*song.rate),end:Math.round(buildEnd*song.rate),meter:meter()}),
        map_build:()=>edit({type:'build',start:Math.round(buildStart*song.rate),end:Math.round(buildEnd*song.rate),count,meter:meter()}),
        map_apply:()=>session.apply(),map_discard:()=>session.discard(),
        map_notation:()=>edit({type:'set_notation',id:selectedId(),meter:meter(),kind,length:parseFraction(extent),following}),
        map_boundary:()=>edit({type:'move_boundary',id:boundary==='start'?measure!.start_id:measure!.end_id,frame:Math.round(boundarySeconds*song.rate)},false),
        map_beat:()=>edit({type:'set_beat',measure:selectedId(),id:beatId||null,frame:Math.round(beatSeconds*song.rate),offset:parseFraction(beatQuarter)},false),
        map_remove_beat:()=>edit({type:'remove_beat',measure:selectedId(),id:beatId},false),
        map_split:()=>edit({type:'split',id:selectedId(),frame:Math.round(beatSeconds*song.rate),left:parseFraction(beatQuarter)}),
        map_merge:()=>edit({type:'merge',id:selectedId(),meter:meter()}),
        map_remove:()=>edit({type:'remove',id:selectedId()}),
        map_protect:()=>edit({type:'set_protected',id:selectedId(),protected:!measure!.protected},false),
        map_respace:()=>edit({type:'respace',first,last}),map_undo:()=>session.history('undo'),map_redo:()=>session.history('redo'),
        map_fit:()=>{if(measure){const start=measure.start/song.rate,end=measure.end/song.rate,pad=(end-start)*.15;view={start:Math.max(0,start-pad),end:Math.min(song.duration,end+pad)};}},
        map_whole:()=>{view={start:0,end:song.duration};}
      };
      await commands[id]?.();
    }catch(e){formError=String(e);}
  }
  const title=(id:ButtonId)=>keymap.title(id);
  const label=(id:ButtonId, fallback:string)=>keymap.buttonLabel(id,fallback);
  const disabled=(id:ButtonId)=>!active||!available(id);
  function select(id:string){selected=id;formError='';}
  function chooseBeat(){const beat=map?.measures.find(m=>m.id===selected)?.beats.find(b=>b.id===beatId);if(beat){beatSeconds=beat.frame/song.rate;beatQuarter=fractionText(beat.offset);}}
</script>
<section class="measure-editor" aria-label="Measure mapping workspace" hidden={!active}>
  <div class="mapping-workspace">
    <div class="mapping-heading"><div><h2>Measure map</h2><p>Fit musical time to the recording.</p></div><span class="map-status">{editState.preview?'Draft preview':measures.length?`${measures.length} measures`:'No map yet'}</span></div>
    <div class="mapping-toolbar" role="group" aria-label="Measure-map tools">
      <button class="button" disabled={disabled('map_analyze')} title={title('map_analyze')} onclick={()=>run('map_analyze')}>{label('map_analyze','Analyze song')}</button>
      <button class="text-button" disabled={disabled('map_undo')} title={title('map_undo')} onclick={()=>run('map_undo')}>{label('map_undo','Undo')}</button>
      <button class="text-button" disabled={disabled('map_redo')} title={title('map_redo')} onclick={()=>run('map_redo')}>{label('map_redo','Redo')}</button>
      <span class="toolbar-spacer"></span>
      <button class="text-button" disabled={disabled('map_fit')} title={title('map_fit')} onclick={()=>run('map_fit')}>{label('map_fit','Fit measure')}</button>
      <button class="text-button" disabled={disabled('map_whole')} title={title('map_whole')} onclick={()=>run('map_whole')}>{label('map_whole','Whole song')}</button>
    </div>
    {#if analysis.status?.running}<div class="analysis-progress" role="status"><span>{analysis.status.phase}…</span><button class="text-button" disabled={disabled('map_cancel_analysis')} title={title('map_cancel_analysis')} onclick={()=>run('map_cancel_analysis')}>Cancel</button></div>{/if}
    {#if analysis.error||analysis.status?.error}<p class="mapping-error" role="alert">{analysis.error||analysis.status?.error}</p>{/if}
    {#if candidates}<div class="analysis-summary"><span>{candidates.beats.length} suggested beats · {candidates.elapsed_seconds.toFixed(1)} s analysis</span><span>Set meter and pulse, then preview measures.</span>{#if candidates.map_revision!==(map?.revision??0)}<span>The map has changed since analysis. Review this draft before applying.</span>{/if}</div>{/if}
    {#if editState.error||formError}<p class="mapping-error" role="alert">{formError||editState.error}</p>{/if}
    {#if editState.preview}<div class="mapping-preview" role="status"><span>Review the green draft. Your saved map is unchanged.</span><button class="button primary" disabled={disabled('map_apply')} title={title('map_apply')} onclick={()=>run('map_apply')}>{label('map_apply','Apply map')}</button><button class="text-button" disabled={disabled('map_discard')} title={title('map_discard')} onclick={()=>run('map_discard')}>{label('map_discard','Discard')}</button></div>{/if}
    <MapTimeline peaks={song.peaks} duration={song.duration} rate={song.rate} position={transport.position} {view} mapView={displayed} candidates={candidates?.beats??[]} {selected} disabled={editState.pending||!!editState.preview} onview={v=>{view=v;}} {onseek} onselect={select} onedit={e=>{void edit(e,false);}}/>
    <div class="mapping-legend"><span><i class="bar"></i>Measure boundary</span><span><i></i>Interpolated beat</span><span><i class="anchor"></i>Fixed beat anchor</span><span>Drag a line to correct timing.</span></div>
  </div>
  <aside class="mapping-inspector" aria-label="Measure-map inspector">
    <details><summary>Automatic draft</summary><p>Beat This! suggests beats and downbeats. Meter and accepted corrections stay yours.</p><button class="button full-width" disabled={disabled('map_install_model')} title={title('map_install_model')} onclick={()=>run('map_install_model')}>{label('map_install_model',analysis.status?.model_ready?'Verify / reinstall model':'Download analysis model (83 MB)')}</button><p>The model uses the MIT license. Installed models work offline. Manual mapping needs no download.</p>{#if candidates}<button class="button primary full-width" disabled={disabled('map_use_analysis')} title={title('map_use_analysis')} onclick={()=>run('map_use_analysis')}>{label('map_use_analysis','Preview detected measures')}</button>{/if}</details>
    <details open><summary>Build measures</summary>
      <p>Set two boundaries and the number of measures between them.</p>
      <div class="mapping-pair"><label>Span start (seconds)<input type="number" min="0" max={song.duration} step="any" bind:value={buildStart}/></label><label>Span end (seconds)<input type="number" min="0" max={song.duration} step="any" bind:value={buildEnd}/></label></div>
      <div class="inline-tools"><button class="text-button" onclick={()=>{buildStart=transport.position;}}>Start at playhead</button><button class="text-button" onclick={()=>{buildEnd=transport.position;}}>End at playhead</button></div>
      <label>Number of measures<input type="number" min="1" max="4096" step="1" bind:value={count}/></label>
      <div class="mapping-pair"><label>Meter numerator<input type="number" min="1" max="99" step="1" bind:value={numerator}/></label><label>Meter denominator<select bind:value={denominator}>{#each [1,2,4,8,16,32,64] as d}<option value={d}>{d}</option>{/each}</select></label></div>
      <label>Pulse note value<select bind:value={pulse}><option value="1">Quarter note</option><option value="1/2">Eighth note</option><option value="3/2">Dotted quarter</option><option value="2">Half note</option><option value="1/4">Sixteenth note</option></select></label>
      <label>Grouping (optional)<input bind:value={grouping} placeholder="2+2+3"/></label>
      <button class="button primary full-width" disabled={disabled('map_build')} title={title('map_build')} onclick={()=>run('map_build')}>{label('map_build','Preview measures')}</button>
    </details>
    <details open><summary>{measure?`Measure ${measure.label}`:'Select a measure'}</summary>
      {#if measure}
        <p>{time(measure.start/song.rate,3)} – {time(measure.end/song.rate,3)}{measure.protected?' · Protected':''}</p>
        <label>Measure kind<select bind:value={kind}><option value="full">Full measure</option><option value="pickup">Pickup</option><option value="incomplete">Incomplete measure</option></select></label>
        {#if kind!=='full'}<label>Actual duration (quarter notes)<input bind:value={extent} placeholder="1 or 3/2"/></label>{/if}
        <label class="mapping-check"><input type="checkbox" bind:checked={following}/>Apply meter until the next meter change</label>
        <p class="mapping-hint">Uses the meter above. Previewing notation rebuilds the internal beat grid.</p>
        <button class="button full-width" disabled={disabled('map_notation')} title={title('map_notation')} onclick={()=>run('map_notation')}>{label('map_notation','Preview notation')}</button>
        <button class="text-button" disabled={disabled('map_protect')} title={title('map_protect')} onclick={()=>run('map_protect')}>{label('map_protect',measure.protected?'Unprotect measure':'Protect measure')}</button>
        <div class="mapping-pair"><label>Boundary<select bind:value={boundary} onchange={()=>{boundarySeconds=(boundary==='start'?measure!.start:measure!.end)/song.rate;}}><option value="start">Start</option><option value="end">End</option></select></label><label>Position (seconds)<input type="number" min="0" max={song.duration} step="any" bind:value={boundarySeconds}/></label></div>
        <button class="button full-width" disabled={disabled('map_boundary')} title={title('map_boundary')} onclick={()=>run('map_boundary')}>{label('map_boundary','Move boundary')}</button>
      {:else}<p>Choose a measure in the ruler to edit its timing, meter, or extent.</p>{/if}
    </details>
    {#if measure}
      <details><summary>Beat anchors and splits</summary>
        <label>Beat anchor<select bind:value={beatId} onchange={chooseBeat}><option value="">New anchor</option>{#each map?.measures.find(m=>m.id===selected)?.beats??[] as b}<option value={b.id}>{fractionText(b.offset)} quarters · {time(b.frame/song.rate,3)}</option>{/each}</select></label>
        <label>Anchor position (seconds)<input type="number" min="0" max={song.duration} step="any" bind:value={beatSeconds}/></label><button class="text-button" onclick={()=>{beatSeconds=transport.position;}}>Use playhead</button>
        <label>Notated offset (quarter notes)<input bind:value={beatQuarter} placeholder="1 or 3/2"/></label>
        <button class="button full-width" disabled={disabled('map_beat')} title={title('map_beat')} onclick={()=>run('map_beat')}>{label('map_beat',beatId?'Update anchor':'Add anchor')}</button>
        <button class="text-button" disabled={!beatId||disabled('map_remove_beat')} title={title('map_remove_beat')} onclick={()=>run('map_remove_beat')}>{label('map_remove_beat','Remove anchor')}</button>
        <p>Split here using the source position and notated offset above.</p>
        <button class="button full-width" disabled={disabled('map_split')} title={title('map_split')} onclick={()=>run('map_split')}>{label('map_split','Preview split')}</button>
        <button class="text-button" disabled={disabled('map_merge')} title={title('map_merge')} onclick={()=>run('map_merge')}>{label('map_merge','Preview merge with next')}</button>
      </details>
      <details><summary>Respace or free time</summary>
        <label>First measure<select bind:value={first}>{#each measures as m}<option value={m.id}>{m.label}</option>{/each}</select></label><label>Last measure<select bind:value={last}>{#each measures as m}<option value={m.id}>{m.label}</option>{/each}</select></label>
        <p>Keep outer boundaries fixed and rebuild spacing by notated duration. Internal anchors are removed.</p>
        <button class="button full-width" disabled={disabled('map_respace')} title={title('map_respace')} onclick={()=>run('map_respace')}>{label('map_respace','Preview spacing')}</button>
        <button class="text-button" disabled={disabled('map_remove')} title={title('map_remove')} onclick={()=>run('map_remove')}>{label('map_remove','Make this measure free time')}</button>
      </details>
    {/if}
  </aside>
</section>
<style>
  .analysis-progress{display:flex;justify-content:space-between;align-items:center;padding:9px 12px;margin-bottom:12px;background:#eef2fa;font-size:12px;color:#566e9c}.analysis-summary{display:flex;gap:7px 15px;flex-wrap:wrap;font-size:11px;color:#7e6b43;margin-bottom:12px}.analysis-summary>span:last-child{font-size:10px}
  .measure-editor{display:grid;grid-template-columns:minmax(0,1fr) 285px;flex:1;min-height:0;background:white;border:1px solid #e0e6f0;border-radius:9px;overflow:hidden}.measure-editor[hidden]{display:none}.mapping-workspace{padding:23px 25px 14px;min-width:0;display:flex;flex-direction:column;min-height:0}.mapping-heading{display:flex;align-items:center;justify-content:space-between;gap:15px;margin-bottom:15px}.mapping-heading h2{font-size:20px;letter-spacing:-.35px;font-weight:500}.mapping-heading p{font-size:12px;color:#7a879f;margin-top:5px}.map-status{font-size:11px;color:#66877a;white-space:nowrap}.mapping-toolbar{display:flex;gap:13px;align-items:center;margin-bottom:17px}.mapping-toolbar .button{padding:8px 12px;font-size:12px}.toolbar-spacer{flex:1}.mapping-inspector{padding:18px 19px;border-left:1px solid #e0e6f0;background:#f9fafc;overflow:auto;min-height:0;font-size:12px}.mapping-inspector details{padding:9px 0 17px;border-bottom:1px solid #e0e6f0}.mapping-inspector summary{font-size:13px;font-weight:600;cursor:pointer;color:#3d506f}.mapping-inspector p{color:#7a879d;line-height:1.5;font-size:11px;margin:9px 0}.mapping-inspector label{display:flex;flex-direction:column;gap:5px;font-size:11px;margin-top:11px;color:#596b86}.mapping-inspector input,.mapping-inspector select{min-width:0;width:100%;padding:7px 8px;border:1px solid #d3ddeb;border-radius:4px;background:#fff;color:#334966;font:inherit;font-size:12px}.mapping-pair{display:grid;grid-template-columns:1fr 1fr;gap:10px}.mapping-check{flex-direction:row!important;align-items:flex-start;line-height:1.45}.mapping-check input{width:auto;margin-top:2px}.full-width{width:100%;margin-top:12px;font-size:12px;padding:9px}.mapping-inspector .text-button{font-size:11px;padding:5px 0;margin-top:4px}.inline-tools{display:flex;gap:13px;justify-content:space-between}.mapping-preview{display:flex;align-items:center;gap:12px;padding:11px 13px;background:#e8f1ec;color:#49695b;font-size:11px;margin-bottom:12px;border-radius:4px}.mapping-preview>span{flex:1}.mapping-preview .button{padding:7px 10px;font-size:11px}.mapping-error{color:#a7475c;background:#fff0f2;font-size:12px;padding:10px 12px;border-radius:4px;margin-bottom:10px}.mapping-legend{display:flex;align-items:center;gap:17px;font-size:10px;color:#7a879f;margin-top:11px;flex-wrap:wrap}.mapping-legend span{display:flex;gap:5px;align-items:center}.mapping-legend i{height:11px;border-left:1px dashed #8c9dbc}.mapping-legend .bar{border-left:2px solid #84a99a}.mapping-legend .anchor{border-left:1px solid #6d82bb}.mapping-legend>span:last-child{margin-left:auto}
  @media(max-width:1000px){.measure-editor{grid-template-columns:minmax(0,1fr) 255px}.mapping-workspace{padding:17px}.mapping-inspector{padding:13px}.mapping-toolbar{gap:9px}.mapping-legend>span:last-child{display:none}}
  @media(max-height:740px){.mapping-workspace{padding-block:12px}.mapping-heading{margin-bottom:8px}.mapping-heading p{display:none}.mapping-toolbar{margin-bottom:10px}.mapping-inspector{padding-top:8px}}
</style>
