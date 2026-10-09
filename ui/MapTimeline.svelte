<script lang="ts">
  import { onMount } from 'svelte';
  import { clamp, time } from './model';
  import { drawWaveform, sourcePoint, panView, overviewView, wheelView } from './timeline';
  import type { View } from './types';
  import type { MapEdit, MapView, MeasureView, GridPoint } from './timing';
  let { peaks, duration, rate, position, view, mapView, selected, disabled = false, candidates = [], onview, onseek, onselect, onedit }:
    { peaks: [number, number][]; duration: number; rate: number; position: number; view: View; mapView: MapView | null; selected: string | null; disabled?: boolean; candidates?: { frame: number; downbeat: boolean }[];
      onview: (view: View, navigation: 'zoom' | 'pan') => void; onseek: (seconds: number) => void; onselect: (id: string) => void; onedit: (edit: MapEdit) => void } = $props();
  let canvas: HTMLCanvasElement, overview: HTMLCanvasElement, field: HTMLDivElement;
  let size = $state(0), width = $state(800), keyboard = $state(false), overviewPointer = $state<number | null>(null);
  type Drag = { pointer: number; x: number; width: number; view: View; edit: Extract<MapEdit, {type:'move_boundary'|'set_beat'}> | null; selected: string | null; pan: boolean };
  let drag = $state<Drag | null>(null), draggedFrame = $state<number | null>(null);
  const measures = $derived(mapView?.measures ?? []);
  const span = $derived(view.end-view.start);
  const pct = (frame: number) => (frame/rate-view.start)/span*100;
  const visible = $derived(measures.filter(m => m.end/rate > view.start && m.start/rate < view.end));
  const startIds = $derived(new Set(measures.map(m=>m.start_id)));
  const measurePixels=(m:MeasureView)=>(m.end-m.start)/rate/span*width;
  function visibleBeats(m:MeasureView){
    if(measurePixels(m)<60)return [];
    let previous=-Infinity;
    return m.grid.filter(p=>{if(p.quarter<=0||p.frame/rate<view.start||p.frame/rate>view.end)return false;const x=pct(p.frame)*width/100;if(x-previous<22&&!p.id)return false;previous=x;return true;});
  }
  const shownCandidates=$derived.by(()=>{
    let last=-Infinity;return candidates.filter(p=>{if(p.frame/rate<view.start||p.frame/rate>view.end)return false;const x=pct(p.frame)*width/100;if(x-last<3&&!p.downbeat)return false;last=x;return true;});
  });
  const ticks = $derived(Array.from({length:7},(_,i)=>view.start+span*i/6));
  const gaps = $derived.by(() => {
    const result: {start:number;end:number}[]=[];let at=0;
    for(const m of measures){if(m.start>at)result.push({start:at,end:m.start});at=m.end;}
    if(at<Math.round(duration*rate))result.push({start:at,end:Math.round(duration*rate)});return result;
  });
  const frame = (event: MouseEvent) => Math.round(sourcePoint(view,duration,field.getBoundingClientRect(),event.clientX)*rate);
  function displayed(id: string, at: number) { return drag?.edit?.type==='move_boundary' && drag.edit.id===id && draggedFrame!==null ? draggedFrame : at; }
  $effect(()=>{void size; drawWaveform(canvas,peaks,duration,view.start,view.end);drawWaveform(overview,peaks,duration,0,duration,true);});
  onMount(()=>{
    const observer=new ResizeObserver(()=>{size++;width=Math.max(1,field.clientWidth);});observer.observe(field);
    const pointer=()=>{keyboard=false;},key=()=>{keyboard=true;};
    window.addEventListener('pointerdown',pointer,true);window.addEventListener('keydown',key,true);
    const wheel=(event:WheelEvent)=>{event.preventDefault();if(!drag&&overviewPointer===null)onview(wheelView(view,duration,field.getBoundingClientRect(),event),'zoom');};
    field.addEventListener('wheel',wheel,{passive:false});
    return()=>{observer.disconnect();field.removeEventListener('wheel',wheel);window.removeEventListener('pointerdown',pointer,true);window.removeEventListener('keydown',key,true);};
  });
  function down(event:PointerEvent){
    if(drag||overviewPointer!==null||![0,1].includes(event.button))return;
    const target=(event.target as HTMLElement).closest<HTMLElement>('[data-boundary],[data-beat]');
    const id=target?.dataset.measure??null;
    let edit:Extract<MapEdit,{type:'move_boundary'|'set_beat'}>|null=null;
    if(target&&id&&event.button===0){
      if(disabled)return;onselect(id);
      if(target.dataset.boundary)edit={type:'move_boundary',id:target.dataset.boundary,frame:frame(event)};
      else if(target.dataset.quarter)edit={type:'set_beat',measure:id,id:target.dataset.beat||null,frame:frame(event),offset:{num:Math.round(Number(target.dataset.quarter)*960),den:960}};
    }
    drag={pointer:event.pointerId,x:event.clientX,width:field.getBoundingClientRect().width,view:{...view},edit,selected:id,pan:event.button===1};
    draggedFrame=edit?frame(event):null;field.setPointerCapture(event.pointerId);if(edit&&target)target.focus();else field.focus();event.preventDefault();
  }
  function move(event:PointerEvent){
    if(!drag||event.pointerId!==drag.pointer)return;
    if(drag.pan)onview(panView(drag.view,duration,event.clientX-drag.x,drag.width),'pan');else if(drag.edit)draggedFrame=frame(event);
  }
  function up(event:PointerEvent){
    if(!drag||event.pointerId!==drag.pointer)return;
    if(!drag.pan){
      if(drag.edit&&Math.abs(event.clientX-drag.x)>2)onedit({...drag.edit,frame:frame(event)});
      else if(!drag.edit)onseek(frame(event)/rate);
    }
    cancel(event);
  }
  function cancel(event:PointerEvent){if(!drag||event.pointerId!==drag.pointer)return;drag=null;draggedFrame=null;if(field.hasPointerCapture(event.pointerId))field.releasePointerCapture(event.pointerId);}
  function nudge(event:KeyboardEvent,m:MeasureView,id:string,at:number,beat?:GridPoint){
    if(!['ArrowLeft','ArrowRight'].includes(event.key))return;event.preventDefault();event.stopPropagation();if(disabled)return;
    const next=clamp(at+Math.round(rate*(event.shiftKey?.001:.01))*(event.key==='ArrowRight'?1:-1),0,Math.round(duration*rate));
    onselect(m.id);onedit(beat?{type:'set_beat',measure:m.id,id:beat.id,frame:next,offset:{num:Math.round(beat.quarter*960),den:960}}:{type:'move_boundary',id,frame:next});
  }
  function overviewPan(event:MouseEvent){onview(overviewView(view,duration,overview.getBoundingClientRect(),event.clientX),'pan');}
  function overviewDown(event:PointerEvent){if(event.button!==0||drag)return;overviewPointer=event.pointerId;const button=event.currentTarget as HTMLElement;button.setPointerCapture(event.pointerId);event.preventDefault();overviewPan(event);}
  function overviewEnd(event:PointerEvent){if(overviewPointer!==event.pointerId)return;overviewPointer=null;const button=event.currentTarget as HTMLElement;if(button.hasPointerCapture(event.pointerId))button.releasePointerCapture(event.pointerId);}
</script>
<div class="map-overview-wrap">
  <button class="map-overview" class:keyboard-focus={keyboard} aria-label="Move measure-map view" title="Click or drag to move the detailed view" onclick={overviewPan}
    onpointerdown={overviewDown} onpointermove={e=>{if(e.pointerId===overviewPointer)overviewPan(e);}} onpointerup={e=>{if(e.pointerId===overviewPointer)overviewPan(e);overviewEnd(e);}} onpointercancel={overviewEnd} onlostpointercapture={overviewEnd}>
    <canvas bind:this={overview}></canvas><span class="map-viewport" style:left={`${view.start/duration*100}%`} style:width={`${span/duration*100}%`}></span>
    <i style:left={`${position/duration*100}%`}></i>
  </button><span>Whole song</span><span>{time(duration,0)}</span>
</div>
<div class="measure-lane" aria-label="Measures on timeline">
  {#each visible as m (m.id)}
    <button class:dense={measurePixels(m)<45} class:selected={selected===m.id} class:protected={m.protected} onclick={()=>onselect(m.id)} title={`Measure ${m.label}, ${m.meter.numerator}/${m.meter.denominator}${m.kind!=='full'?`, ${m.kind}`:''}${m.protected?', protected':''}`}
      style:left={`${Math.max(0,pct(m.start))}%`} style:width={`${Math.min(100,pct(m.end))-Math.max(0,pct(m.start))}%`}>
      <strong>{m.label}</strong><span>{m.meter.numerator}/{m.meter.denominator}{m.kind!=='full'?' · partial':''}{m.protected?' · protected':''}</span>
    </button>
  {/each}
</div>
<div class="map-wave-field" class:keyboard-focus={keyboard} class:panning={drag?.pan} bind:this={field} role="region" tabindex="-1" aria-label="Measure-map waveform. Click to seek; drag beat and measure lines to correct timing."
  onpointerdown={down} onpointermove={move} onpointerup={up} onpointercancel={cancel} onlostpointercapture={cancel} onauxclick={e=>{if(e.button===1)e.preventDefault();}}>
  <div class="map-seek" role="slider" tabindex="0" aria-label="Measure-map song position" aria-valuemin={0} aria-valuemax={duration} aria-valuenow={position}
    onkeydown={e=>{if(['ArrowLeft','ArrowRight','Home','End'].includes(e.key)){e.preventDefault();e.stopPropagation();onseek(e.key==='Home'?0:e.key==='End'?duration:clamp(position+(e.key==='ArrowRight'?1:-1)*(e.shiftKey?.1:5),0,duration));}}}></div>
  <div class="map-time-ruler" aria-hidden="true">{#each ticks as tick,i}<span style:left={`${i/6*100}%`}>{time(tick)}</span>{/each}</div>
  <canvas class="map-wave-canvas" bind:this={canvas}></canvas>
  {#each gaps.filter(g=>g.end/rate>view.start&&g.start/rate<view.end) as gap}
    <div class="free-time" style:left={`${Math.max(0,pct(gap.start))}%`} style:width={`${Math.min(100,pct(gap.end))-Math.max(0,pct(gap.start))}%`}><span>Unmapped / free time</span></div>
  {/each}
  {#each shownCandidates as p}
    <div class="candidate-line" class:downbeat={p.downbeat} style:left={`${pct(p.frame)}%`} aria-hidden="true"></div>
  {/each}
  {#each visible as m (m.id)}
    {#if selected===m.id}<div class="selected-measure" style:left={`${Math.max(0,pct(m.start))}%`} style:width={`${Math.min(100,pct(m.end))-Math.max(0,pct(m.start))}%`}></div>{/if}
    {#if m.start/rate>=view.start && (measurePixels(m)>=16 || selected===m.id)}
      <button class="measure-handle" data-boundary={m.start_id} data-measure={m.id} style:left={`${pct(displayed(m.start_id,m.start))}%`} aria-label={`Measure ${m.label} start`} aria-disabled={disabled||m.protected}
        title="Drag measure boundary; arrow keys adjust; Shift gives fine adjustment" onclick={()=>onselect(m.id)} onkeydown={e=>nudge(e,m,m.start_id,m.start)}><span>{m.label}</span></button>
    {/if}
    {#each visibleBeats(m) as p}
      <button class="beat-handle" class:anchor={!!p.id} data-beat={p.id??''} data-quarter={p.quarter} data-measure={m.id}
        style:left={`${pct(drag?.edit?.type==='set_beat'&&drag.edit.measure===m.id&&drag.edit.offset.num/drag.edit.offset.den===p.quarter&&draggedFrame!==null?draggedFrame:p.frame)}%`}
        aria-label={`Measure ${m.label} beat ${p.label}`} aria-disabled={disabled||m.protected} title={`${p.id?'Fixed anchor':'Interpolated beat'}: drag to correct timing`}
        onclick={()=>onselect(m.id)} onkeydown={e=>nudge(e,m,'',p.frame,p)}><span>{p.label}</span></button>
    {/each}
    {#if !startIds.has(m.end_id)&&m.end/rate>=view.start&&m.end/rate<=view.end}
      <button class="measure-handle end" data-boundary={m.end_id} data-measure={m.id} style:left={`${pct(displayed(m.end_id,m.end))}%`} aria-label={`Measure ${m.label} end`} aria-disabled={disabled||m.protected}
        onclick={()=>onselect(m.id)} onkeydown={e=>nudge(e,m,m.end_id,m.end)}><span>End</span></button>
    {/if}
  {/each}
  {#if position>=view.start&&position<=view.end}<div class="map-playhead" style:left={`${(position-view.start)/span*100}%`} aria-hidden="true"></div>{/if}
</div>
<style>
  .map-overview-wrap{position:relative;padding-bottom:23px;flex-shrink:0}.map-overview{position:relative;display:block;width:100%;height:46px;padding:5px 0;border:0;border-radius:5px;background:#e8edf6;overflow:hidden;touch-action:none;cursor:grab}.map-overview canvas{width:100%;height:100%;display:block}.map-viewport{position:absolute;inset-block:0;border:1px solid #6d82bb;background:#526fc013;min-width:3px;pointer-events:none}.map-overview i{position:absolute;inset-block:0;border-left:2px solid #526fc0}.map-overview-wrap>span{position:absolute;bottom:4px;font-size:11px;color:#758098}.map-overview-wrap>span:last-child{right:0}
  .measure-lane{position:relative;height:44px;background:#edf1f7;border-radius:5px 5px 0 0;overflow:hidden;flex-shrink:0}.measure-lane button{position:absolute;inset-block:0;border:0;border-right:1px solid #fff;text-align:left;background:#dce8e2;color:#334c47;padding:4px 9px;overflow:hidden;white-space:nowrap}.measure-lane button.dense{padding-inline:1px}.measure-lane button.dense span{display:none}.measure-lane button.selected{background:#a7c9bd}.measure-lane button.protected{box-shadow:inset 0 -3px #6b8c80}.measure-lane strong{display:block;font-size:13px}.measure-lane span{font-size:10px}
  .map-wave-field{position:relative;flex:1;min-height:230px;background:#233552;border-radius:0 0 6px 6px;overflow:hidden;touch-action:none;user-select:none;cursor:crosshair}.map-wave-field.panning{cursor:grabbing}.map-seek{position:absolute;inset:0;outline-offset:-3px}.map-wave-canvas{position:absolute;left:0;top:70px;width:100%;height:calc(100% - 86px);pointer-events:none}.map-time-ruler{position:absolute;inset:0 0 auto;height:30px;color:#bbcae2;font-size:11px;pointer-events:none}.map-time-ruler span{position:absolute;top:8px;transform:translateX(-50%)}.map-time-ruler span:first-child{transform:none;padding-left:8px}.map-time-ruler span:last-child{transform:translateX(-100%);padding-right:8px}
  .free-time{position:absolute;inset-block:30px 0;background:repeating-linear-gradient(135deg,#ffffff06,#ffffff06 5px,transparent 5px,transparent 12px);pointer-events:none}.free-time span{position:absolute;bottom:10px;left:8px;font-size:10px;color:#a7b6d0;white-space:nowrap}.selected-measure{position:absolute;inset-block:30px 0;background:#a7c9bd13;pointer-events:none}.measure-handle{position:absolute;top:30px;bottom:0;width:14px;padding:0;border:0;border-left:2px solid #a7c9bd;border-radius:0;background:transparent;color:#233552;cursor:ew-resize;touch-action:none}.measure-handle span{position:absolute;top:0;left:-2px;min-width:22px;padding:3px 5px;background:#a7c9bd;border-radius:0 0 3px 0;font-size:11px;font-weight:600}.measure-handle.end{transform:translateX(-12px);border-left:0;border-right:2px solid #a7c9bd}.measure-handle.end span{left:auto;right:-2px}.beat-handle{position:absolute;top:57px;bottom:0;width:10px;padding:0;border:0;border-left:1px dashed #adbcdb75;border-radius:0;background:transparent;color:#adbcdb;cursor:ew-resize;touch-action:none}.beat-handle span{position:absolute;top:0;left:3px;font-size:10px}.beat-handle.anchor{border-left:1px solid #cad9f0}.candidate-line{position:absolute;inset-block:74px 0;border-left:1px dotted #dfbb7f99;pointer-events:none}.candidate-line.downbeat{border-left:2px dotted #dfbb7f}.map-playhead{position:absolute;inset-block:26px 0;border-left:2px solid white;pointer-events:none}
  .map-overview:not(.keyboard-focus):focus,.map-wave-field:not(.keyboard-focus):focus,.map-wave-field:not(.keyboard-focus) :focus{outline:none}.map-seek:focus-visible{outline:3px solid #8297d2}.measure-handle:focus-visible,.beat-handle:focus-visible{outline:2px solid #fff;outline-offset:2px}
  @media(max-height:740px){.map-overview{height:32px}.map-wave-field{min-height:170px}}
</style>
