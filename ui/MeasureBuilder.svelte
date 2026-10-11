<script lang="ts">
  import {untrack} from 'svelte';
  import MeterFields from './MeterFields.svelte';
  import {meterForm,readMeter} from './meter-form';
  import type {Candidates,MapEdit,MapPreview} from './timing';
  import type {Song} from './types';
  import type {ButtonId,MappingActionId} from './actions';
  import type {Keymap} from './keymap';
  import {time} from './model';
  let {song,candidates,resultId,analysisRunning,position,preview,pending,keymap,onpreview,run,available,epsilonMs=$bindable(30)}:
    {epsilonMs?:number;song:Song;candidates:Candidates|null;resultId:number;analysisRunning:boolean;position:number;preview:MapPreview|null;pending:boolean;keymap:Keymap;onpreview:(edit:MapEdit)=>Promise<void>;run:(id:ButtonId)=>void;available:(id:ButtonId)=>boolean}=$props();
  let source=$state<'detected'|'manual'>('manual');
  let start=$state(0),end=$state(untrack(()=>song.duration)),count=$state(untrack(()=>Math.max(1,Math.round(song.duration/2))));
  let notation=$state(meterForm()),firstDownbeat=$state(0),lastSignature=$state('');
  let lastResultId=-1;
  $effect(()=>{
    if(candidates&&resultId!==lastResultId){lastResultId=resultId;source='detected';const within=candidates.beats.filter(b=>b.frame>=Math.round(start*song.rate)&&b.frame<=Math.round(end*song.rate));const first=within.find(b=>b.downbeat)??within[0];if(first)firstDownbeat=first.frame/song.rate;}
  });
  const summary=$derived(preview?.draft??null);
  const previewAction=$derived(source==='detected'?'map_use_analysis':'map_build');
  const signature=()=>JSON.stringify({source,start,end,count,notation,firstDownbeat,epsilonMs});
  export function isStale(){return !!summary&&lastSignature!==signature();}
  export function canRun(id:MappingActionId){
    if(pending)return false;
    if(id==='map_use_analysis'||id==='map_first_downbeat')return !!candidates&&!analysisRunning;
    return true;
  }
  export async function command(id:MappingActionId){
    if(!canRun(id))return;
    if(id==='map_span_start'){start=position;return;}
    if(id==='map_span_end'){end=position;return;}
    if(id==='map_first_downbeat'){firstDownbeat=position;return;}
    const meter=readMeter(notation);
    if(id==='map_use_analysis'){
      if(!Number.isFinite(epsilonMs)||epsilonMs<0||epsilonMs>1000)throw new Error('Beat timing tolerance must be between 0 and 1000 ms.');
      source='detected';lastSignature=signature();
      await onpreview({type:'build_from_pulses',pulses:candidates!.beats.map(b=>b.frame),downbeats:candidates!.beats.filter(b=>b.downbeat).map(b=>b.frame),first_downbeat:Math.round(firstDownbeat*song.rate),epsilon_ms:epsilonMs,start:Math.round(start*song.rate),end:Math.round(end*song.rate),meter});
    }else if(id==='map_build'){
      source='manual';lastSignature=signature();
      await onpreview({type:'build',start:Math.round(start*song.rate),end:Math.round(end*song.rate),count,meter});
    }
  }
  const label=(id:ButtonId,fallback:string)=>keymap.buttonLabel(id,fallback);
</script>
<details open class="measure-builder"><summary>Create measures</summary>
  <fieldset disabled={pending}>
    <label>Create measures from<select bind:value={source} aria-label="Measure source"><option value="detected" disabled={!candidates}>Detected beats</option><option value="manual">Even spacing (manual)</option></select></label>
    {#if source==='detected'}<p>Group detected pulses using the time signature and pulse note value. Measure boundaries follow the detected beats.</p>
    {:else}<p>Divide a span evenly. This mode does not use detected beats. Analyze the song to align a draft with its rhythm.</p>{/if}
    <MeterFields bind:value={notation} prefix="Draft " showGrouping={false}/>
    {#if source==='detected'}
      <label>First downbeat (seconds)<input type="number" min={start} max={end} step="any" bind:value={firstDownbeat}/></label>
      <button class="text-button" disabled={!available('map_first_downbeat')} title={keymap.title('map_first_downbeat')} onclick={()=>run('map_first_downbeat')}>{label('map_first_downbeat','Use playhead for first downbeat')}</button>
      <p>The first measure starts on the detected beat nearest this position. Its location sets which beats begin the measures.</p>
      <label>Beat timing tolerance ε (ms)<input aria-label="Beat timing tolerance (ms)" type="number" min="0" max="1000" step="any" bind:value={epsilonMs}/></label>
      <p>Beats are interpolated unless a timing correction is needed to stay within this error of the detected pulses. 0 ms fits pulses exactly. Measure boundaries always follow detection.</p>
    {:else}<label>Number of measures<input type="number" min="1" max="4096" step="1" bind:value={count}/></label>{/if}
    {#if isStale()}<p class="changed" role="status">Settings changed. Preview again before applying.</p>{/if}
    <button class="button primary preview" disabled={!available(previewAction)} title={keymap.title(previewAction)} onclick={()=>run(previewAction)}>{label(previewAction,source==='detected'?'Preview detected measures':'Preview evenly spaced measures')}</button>
    {#if source==='detected'}
      {#if summary}
        <div class="draft-count" role="status"><output aria-label="Derived measure count">{summary.measure_count} measures</output><span>{summary.pulse_count} detected pulses, including the end boundary</span><span>{summary.pulses_per_measure} pulse intervals per full measure</span><span>{summary.full_measures} full{summary.partial_pulses?` + 1 final partial (${summary.partial_pulses} ${summary.partial_pulses===1?'interval':'intervals'})`:''}</span><span>First measure at {time(summary.first_frame/song.rate,3)}</span><span>{summary.anchor_count} timing {summary.anchor_count===1?'anchor':'anchors'} retained · ε {summary.epsilon_ms} ms</span></div>
      {:else}<p>Measure count is calculated from the detected pulses when you preview. No manual count is needed.</p>{/if}
    {/if}
    <details class="advanced"><summary>Span: {time(start)} – {time(end)}</summary>
    <div class="pair"><label>Span start (seconds)<input type="number" min="0" max={song.duration} step="any" bind:value={start}/></label><label>Span end (seconds)<input type="number" min="0" max={song.duration} step="any" bind:value={end}/></label></div>
    <div class="inline"><button class="text-button" disabled={!available('map_span_start')} title={keymap.title('map_span_start')} onclick={()=>run('map_span_start')}>{label('map_span_start','Start at playhead')}</button><button class="text-button" disabled={!available('map_span_end')} title={keymap.title('map_span_end')} onclick={()=>run('map_span_end')}>{label('map_span_end','End at playhead')}</button></div>
      <label>Grouping (optional)<input aria-label="Draft Grouping (optional)" bind:value={notation.grouping} placeholder="2+2+3"/></label>
    </details>
    <p>Review the green draft, then Apply map.{#if source==='detected'} Audio after the last detected pulse stays unmapped; a final partial measure may need correction.{/if}</p>
  </fieldset>
</details>
<style>
  .measure-builder{padding:9px 0 17px;border-bottom:1px solid #e0e6f0}summary{font-size:13px;font-weight:600;cursor:pointer;color:#3d506f}fieldset{border:0;padding:0;margin:0;min-width:0}p{color:#7a879d;line-height:1.5;font-size:11px;margin:9px 0}label{display:flex;flex-direction:column;gap:5px;font-size:11px;margin-top:11px;color:#596b86}input,select{min-width:0;width:100%;padding:7px 8px;border:1px solid #d3ddeb;border-radius:4px;background:#fff;color:#334966;font:inherit;font-size:12px}.pair{display:grid;grid-template-columns:1fr 1fr;gap:10px}.inline{display:flex;gap:13px;justify-content:space-between}.text-button{font-size:11px;padding:5px 0;margin-top:4px}.preview{width:100%;margin-top:12px;font-size:12px;padding:9px}.draft-count{background:#e8f1ec;border-left:2px solid #84a99a;padding:10px 11px;margin-top:12px;display:flex;flex-direction:column;gap:5px;font-size:11px;color:#49695b}.draft-count output{font-size:15px;font-weight:600}.changed{color:#8a6b36}.advanced{margin-top:12px;padding-block:9px;border-top:1px solid #e0e6f0}.advanced summary{font-size:11px;font-weight:500}
</style>
