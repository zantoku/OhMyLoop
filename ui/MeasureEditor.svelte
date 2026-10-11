<script lang="ts">
  import { untrack, onDestroy, type Snippet } from 'svelte';
  import { AnalysisSession, type AnalysisState } from './analysis-session';
  import AnalysisProgress from './AnalysisProgress.svelte';
  import Icon from './Icon.svelte';
  import MeasureBuilder from './MeasureBuilder.svelte';
  import BatchTools from './BatchTools.svelte';
  import MeasureTools from './MeasureTools.svelte';
  import MapTimeline from './MapTimeline.svelte';
  import {native} from './bridge';
  import MetronomeControls from './MetronomeControls.svelte';
  import {pageForPlayhead} from './timeline';
  import { MappingSession, type MappingState } from './mapping-session';
  import { measureAtFrame, type MapEdit, type MapPreview, type MeasureMap, type MapView } from './timing';
  import type { Song, Transport, View } from './types';
  import type { Keymap } from './keymap';
  import type { ButtonId, MappingActionId } from './actions';
  import { zoom } from './model';
  let { song, map, mapView, transport, keymap, active, clickEnabled=$bindable(true), clickLevel=$bindable(50), onreview, playback, oncommit, onseek, run, available }:
    { song: Song; map: MeasureMap | null; mapView: MapView | null; transport: Transport; keymap: Keymap; active: boolean; clickEnabled?:boolean; clickLevel?:number; onreview:(map:MeasureMap|null)=>void; playback?:Snippet;
      oncommit: (value: MapPreview, songKey:string) => Promise<void>; onseek: (seconds:number)=>void; run:(id:ButtonId)=>void; available:(id:ButtonId)=>boolean } = $props();
  let editState = $state<MappingState>({pending:false,error:'',preview:null,undo:0,redo:0});
  let analysis = $state<AnalysisState>({status:null,candidates:null,requesting:false,error:''});
  const analyzer = new AnalysisSession(()=>({key:song.key,revision:map?.revision??0}),value=>{analysis=value;});
  const candidates = $derived(analysis.candidates?.song_key===song.key?analysis.candidates:null);
  let followingPlayhead=$state(true);
  let epsilonMs=$state(30),resnap=$state(true);
  const audibleMap=$derived(editState.preview?.map??map);
  let detectedVisibility=$state<boolean|null>(null);
  const showDetected=$derived(detectedVisibility??!audibleMap?.measures.length);
  $effect(()=>{
    if(active)onreview(audibleMap?$state.snapshot(audibleMap):null);
  });
  let builder=$state<MeasureBuilder>();
  let tools=$state<MeasureTools>();
  let batch=$state<BatchTools>();
  const toolActions:readonly MappingActionId[]=['map_notation','map_boundary','map_beat','map_remove_beat','map_split','map_merge','map_shift_left','map_shift_right','map_shift_resnap','map_remove','map_reclaim','map_protect','map_anchor_playhead'];
  const builderActions:readonly MappingActionId[]=['map_build','map_use_analysis','map_span_start','map_span_end','map_first_downbeat'];
  let automaticOpen=$state(false);
  let automaticInitialized=false;
  $effect(()=>{if(analysis.status&&!automaticInitialized){automaticInitialized=true;automaticOpen=!analysis.status.model_ready;}});
  $effect(()=>{
    if(!active)return;
    void analyzer.refresh();const timer=setInterval(()=>{if(analysis.status?.running||analysis.requesting)void analyzer.refresh();},400);
    return()=>clearInterval(timer);
  });
  onDestroy(()=>{analyzer.dispose();session.dispose();});
  const session = new MappingSession(()=>({key:song.key,map:map?$state.snapshot(map):null}),value=>oncommit(value,song.key),next=>{editState=next;});
  let view = $state<View>(untrack(()=>({start:0,end:song.duration})));
  export function revealPlayhead(position:number){view=pageForPlayhead(view,song.duration,position);}
  $effect(()=>{if(active&&followingPlayhead&&transport.playing)view=pageForPlayhead(view,song.duration,transport.position);});
  let selected = $state<string|null>(untrack(()=>mapView?.measures[0]?.id??null));
  const displayed = $derived(editState.preview?.view??mapView);
  const measures = $derived(displayed?.measures??[]);
  const measure = $derived(measures.find(m=>m.id===selected)??null);
  let formError=$state('');
  $effect(()=>{const current=measures;untrack(()=>{if(!current.some(m=>m.id===selected))selected=current[0]?.id??null;});});
  $effect(()=>{if(active&&transport.playing)selected=measureAtFrame(measures,Math.round(transport.position*song.rate))?.id??null;});
  async function edit(value:MapEdit,preview=true){
    const origin=value.type==='shift'?measures.find(m=>m.id===value.id)?.start:undefined;
    formError='';await session.edit(value,preview);
    if(value.type==='shift'&&!editState.error&&origin!==undefined&&!measures.some(m=>m.id===value.id))select(measureAtFrame(measures,origin)?.id??measures[0]?.id??null);
  }
  export function canRun(id:MappingActionId):boolean{
    if(id==='map_follow'||id==='map_show_detected')return true;
    if(id==='map_metronome')return native&&(clickEnabled||(!transport.audio_error&&!!audibleMap?.measures.length));
    if(editState.pending)return false;
    if(id==='map_apply')return !!editState.preview&&!builder?.isStale();
    if(id==='map_discard')return !!editState.preview;
    if(id==='map_clear')return !!map?.measures.length||!!editState.preview;
    if(id==='map_origin')return !!map?.measures.length&&!editState.preview;
    if(id==='map_grouping')return !!map?.measures.length&&!editState.preview&&(batch?.canRun(id)??false);
    if(id==='map_notation')return !!measure&&!measure.protected&&!editState.preview;
    if(id==='map_respace')return false;
    if(['map_beat','map_remove_beat','map_split','map_merge','map_shift_left','map_shift_right','map_shift_resnap','map_remove','map_reclaim','map_anchor_playhead'].includes(id))return !editState.preview&&(tools?.canRun(id)??false);
    if(builderActions.includes(id))return builder?.canRun(id)??false;
    if(id==='map_undo')return editState.undo>0;if(id==='map_redo')return editState.redo>0;
    if(['map_boundary','map_protect'].includes(id))return !!measure&&!editState.preview;
    if(id==='map_fit')return !!measure;
    if(id==='map_analyze')return !!analysis.status?.model_ready&&!analysis.status.running&&!analysis.requesting&&!editState.preview;
    if(id==='map_cancel_analysis')return !!analysis.status?.running&&!analysis.requesting;
    if(id==='map_install_model')return !analysis.status?.running&&!analysis.requesting;
    return true;
  }
  export async function command(id:MappingActionId){
    if(!canRun(id))return;
    try{
      if(builderActions.includes(id)){await builder?.command(id);return;}
      if(id==='map_origin'||id==='map_grouping'){await batch?.command(id);return;}
      if(toolActions.includes(id)){await tools?.command(id);return;}
      const commands:Partial<Record<MappingActionId,()=>void|Promise<void>>>={
        map_metronome:()=>{clickEnabled=!clickEnabled;},map_follow:()=>{followingPlayhead=!followingPlayhead;},
        map_show_detected:()=>{detectedVisibility=!showDetected;},
        map_clear:()=>map?.measures.length?edit({type:'clear'},false):session.discard(),
        map_analyze:()=>analyzer.start(),map_cancel_analysis:()=>analyzer.cancel(),map_install_model:()=>analyzer.start(true),
        map_apply:()=>session.apply(),map_discard:()=>session.discard(),
        map_undo:()=>session.history('undo'),map_redo:()=>session.history('redo'),
        map_fit:()=>{if(measure){const start=measure.start/song.rate,end=measure.end/song.rate,pad=(end-start)*.15;view={start:Math.max(0,start-pad),end:Math.min(song.duration,end+pad)};}},
        map_whole:()=>{view={start:0,end:song.duration};}
      };
      await commands[id]?.();
    }catch(e){formError=String(e);}
  }
  const title=(id:ButtonId)=>keymap.title(id);
  const label=(id:ButtonId, fallback:string)=>keymap.buttonLabel(id,fallback);
  const disabled=(id:ButtonId)=>!active||!available(id);
  function select(id:string|null){selected=id;formError='';}
  export function selectMeasure(id:string){
    select(id);const m=measures.find(m=>m.id===id);if(!m)return;
    const start=m.start/song.rate,end=m.end/song.rate;
    if(start<view.start||end>view.end)view=zoom(view,song.duration,Math.max(1,(end-start)/(view.end-view.start)),(start+end)/2);
  }

</script>
<section class="measure-editor" aria-label="Measure mapping workspace" hidden={!active}>
  <div class="mapping-workspace">
    <div class="mapping-heading"><div class="screen-title"><button class="icon-button screen-close" aria-label={label('show_practice','Close measure map')} title={title('show_practice')} disabled={disabled('show_practice')} onclick={()=>run('show_practice')}><Icon name="close"/></button><h2>Measure map</h2></div><span class="map-status">{editState.preview?'Draft preview':measures.length?`${measures.length} measures`:'No map yet'}</span></div>
    <div class="mapping-toolbar" role="group" aria-label="Measure-map tools">
      <button class="text-button" disabled={disabled('map_undo')} title={title('map_undo')} onclick={()=>run('map_undo')}>{label('map_undo','Undo')}</button>
      <button class="text-button" disabled={disabled('map_redo')} title={title('map_redo')} onclick={()=>run('map_redo')}>{label('map_redo','Redo')}</button>
      <button class="text-button" disabled={disabled('map_clear')} title={title('map_clear')} onclick={()=>run('map_clear')}>{label('map_clear','Clear map')}</button>
      <span class="toolbar-spacer"></span>
      <button class="text-button" disabled={disabled('map_fit')} title={title('map_fit')} onclick={()=>run('map_fit')}>{label('map_fit','Fit measure')}</button>
      <button class="text-button" disabled={disabled('map_whole')} title={title('map_whole')} onclick={()=>run('map_whole')}>{label('map_whole','Whole song')}</button>
    </div>
    <div class="review-listening" role="group" aria-label="Map review listening">
      {#if active}<MetronomeControls enabled={clickEnabled} bind:level={clickLevel} {keymap} button="map_metronome" available={!disabled('map_metronome')} run={()=>run('map_metronome')} {native}/>{/if}
      <button class="text-button" aria-pressed={followingPlayhead} disabled={disabled('map_follow')} title={title('map_follow')} onclick={()=>run('map_follow')}>{label('map_follow',followingPlayhead?'Follow playhead on':'Follow playhead off')}</button>
    </div>
    {#if editState.error||formError}<p class="mapping-error" role="alert">{formError||editState.error}</p>{/if}
    {#if editState.preview}<div class="mapping-preview" role="status"><span>Review the green draft. Your saved map is unchanged.</span><button class="button primary" disabled={disabled('map_apply')} title={title('map_apply')} onclick={()=>run('map_apply')}>{label('map_apply','Apply map')}</button><button class="text-button" disabled={disabled('map_discard')} title={title('map_discard')} onclick={()=>run('map_discard')}>{label('map_discard','Discard')}</button></div>{/if}
    {#each editState.preview?.warnings??[] as warning}<p class="analysis-summary" role="status">{warning}</p>{/each}
    <MapTimeline peaks={song.peaks} duration={song.duration} rate={song.rate} position={transport.position} {view} mapView={displayed} candidates={showDetected?(candidates?.beats??[]):[]} {playback} {selected} disabled={editState.pending||!!editState.preview} onview={v=>{view=v;}} {onseek} onselect={select} ongap={frame=>{selected=null;tools?.selectFreeTime(frame);}} onedit={e=>{void edit(e,false);}}/>
    <div class="mapping-legend"><span><i class="bar"></i>Measure boundary</span><span><i></i>Interpolated beat</span><span><i class="anchor"></i>Fixed beat anchor</span><span>Drag a line to correct timing.</span></div>
  </div>
  <aside class="mapping-inspector" aria-label="Measure-map inspector">
    <details open class="analysis-panel"><summary>Analysis</summary>
      <button class="button full-width" disabled={disabled('map_analyze')} title={title('map_analyze')} onclick={()=>run('map_analyze')}>{label('map_analyze','Analyze song')}</button>
      <label class="mapping-check" title={title('map_show_detected')}><input type="checkbox" checked={showDetected} onchange={()=>run('map_show_detected')} disabled={disabled('map_show_detected')}/>{label('map_show_detected','Show detected beats')}</label>
    {#if analysis.status?.running}<AnalysisProgress status={analysis.status}><button class="text-button" disabled={disabled('map_cancel_analysis')} title={title('map_cancel_analysis')} onclick={()=>run('map_cancel_analysis')}>{label('map_cancel_analysis','Cancel')}</button></AnalysisProgress>{/if}
    {#if analysis.error||analysis.status?.error}<p class="mapping-error" role="alert">{analysis.error||analysis.status?.error}</p>{/if}
    {#if candidates}<div class="analysis-summary"><span>{candidates.beats.length} suggested beats · {candidates.elapsed_seconds.toFixed(1)} s analysis</span><span>Use Create measures to group these beats into a draft.</span>{#if candidates.map_revision!==(map?.revision??0)}<span>The map has changed since analysis. Review this draft before applying.</span>{/if}</div>{/if}
    <details bind:open={automaticOpen}><summary>Analysis model</summary><p>Beat This! suggests beats and downbeats. Meter and accepted corrections stay yours.</p><button class="button full-width" disabled={disabled('map_install_model')} title={title('map_install_model')} onclick={()=>run('map_install_model')}>{label('map_install_model',analysis.status?.model_ready?'Verify / reinstall model':'Download analysis model (83 MB)')}</button>{#if analysis.status?.model_path}<p class="model-location">{analysis.status.model_ready?'Installed model folder':'Model download folder'}<code title={analysis.status.model_path}>{analysis.status.model_path}</code></p>{/if}<p>The model uses the MIT license. Installed models work offline. Manual mapping needs no download.</p></details>
    </details>
    <MeasureBuilder bind:epsilonMs bind:this={builder} {song} {candidates} resultId={analysis.status?.id??0} analysisRunning={!!analysis.status?.running} position={transport.position} preview={editState.preview} pending={editState.pending} {keymap} onpreview={edit} {run} {available}/>
    <BatchTools bind:this={batch} songKey={song.key} {map} editable={active&&!editState.pending&&!editState.preview} {keymap} {run} {available} onedit={edit}/>
  </aside>
  <MeasureTools bind:resnap bind:this={tools} {candidates} bind:epsilonMs {song} map={audibleMap} mapView={displayed} {measure} position={transport.position} {keymap} {active} {run} {available} onedit={edit} onselect={select}/>
</section>
<style>
  .review-listening{display:flex;gap:15px;align-items:center;flex-wrap:wrap;flex-shrink:0;margin-bottom:8px;font-size:11px;color:var(--text-secondary)}.review-listening .text-button{margin-left:auto;font-size:11px}
  .analysis-panel .analysis-summary{display:block;margin-top:10px}.analysis-panel .analysis-summary span{display:block;margin-top:5px}.analysis-panel details{border-bottom:0;padding-bottom:0;margin-top:15px}.analysis-panel details summary{font-size:11px;font-weight:500}.analysis-panel .mapping-error{margin-top:10px}
  .model-location code{display:block;margin-top:5px;overflow-wrap:anywhere;user-select:text;color:var(--text-secondary);font-size:11px;line-height:1.5}
  .analysis-summary{display:flex;gap:7px 15px;flex-wrap:wrap;font-size:11px;color:var(--amber-text);margin-bottom:12px}.analysis-summary>span:last-child{font-size:10px}
  .measure-editor{display:grid;grid-template-columns:minmax(0,1fr) 285px;grid-template-rows:minmax(0,1fr) auto;flex:1;min-height:0;background:var(--surface);border:1px solid var(--border);border-radius:9px;overflow:hidden}.measure-editor[hidden]{display:none}.mapping-workspace{padding:16px 25px 14px;min-width:0;overflow:auto;display:flex;flex-direction:column;min-height:0}.mapping-heading{display:flex;align-items:center;justify-content:space-between;gap:15px;margin-bottom:10px}.mapping-heading h2{font-size:20px;letter-spacing:-.35px;font-weight:500}.map-status{font-size:11px;color:var(--success-text);white-space:nowrap}.mapping-toolbar{display:flex;gap:13px;align-items:center;margin-bottom:10px}.toolbar-spacer{flex:1}.mapping-inspector{padding:18px 19px;border-left:1px solid var(--border);background:var(--sidebar);overflow:auto;min-height:0;font-size:12px}.mapping-inspector details{padding:9px 0 17px;border-bottom:1px solid var(--border)}.mapping-inspector summary{font-size:13px;font-weight:600;cursor:pointer;color:var(--text)}.mapping-inspector p{color:var(--muted);line-height:1.5;font-size:11px;margin:9px 0}.mapping-inspector label{display:flex;flex-direction:column;gap:5px;font-size:11px;margin-top:11px;color:var(--text-secondary)}.mapping-inspector input{min-width:0;width:100%;padding:7px 8px;border:1px solid var(--border-strong);border-radius:4px;background:var(--surface);color:var(--text);font:inherit;font-size:12px}.mapping-check{flex-direction:row!important;align-items:flex-start;line-height:1.45}.mapping-check input{width:auto;margin-top:2px}.full-width{width:100%;margin-top:12px;font-size:12px;padding:9px}.mapping-inspector .text-button{font-size:11px;padding:5px 0;margin-top:4px}.mapping-preview{display:flex;align-items:center;gap:12px;padding:11px 13px;background:var(--success-bg);color:var(--success-text);font-size:11px;margin-bottom:12px;border-radius:4px}.mapping-preview>span{flex:1}.mapping-preview .button{padding:7px 10px;font-size:11px}.mapping-error{color:var(--error-text);background:var(--error-bg);font-size:12px;padding:10px 12px;border-radius:4px;margin-bottom:10px}.mapping-legend{display:flex;align-items:center;gap:17px;font-size:10px;color:var(--muted);margin-top:11px;flex-wrap:wrap}.mapping-legend span{display:flex;gap:5px;align-items:center}.mapping-legend i{height:11px;border-left:1px dashed #8c9dbc}.mapping-legend .bar{border-left:2px solid var(--success-border)}.mapping-legend .anchor{border-left:1px solid var(--blue)}.mapping-legend>span:last-child{margin-left:auto}
  @media(max-width:1000px){.measure-editor{grid-template-columns:minmax(0,1fr) 255px}.mapping-workspace{padding:17px}.mapping-inspector{padding:13px}.mapping-toolbar{gap:9px}.mapping-legend>span:last-child{display:none}}
  @media(max-height:740px){.mapping-workspace{padding-block:12px}.mapping-heading{margin-bottom:8px}.mapping-toolbar{margin-bottom:10px}.mapping-inspector{padding-top:8px}}
</style>
