<script lang="ts">
  import { markersForView } from './marker-density';
  import { time } from './model';
  import { measureAtFrame, type MapView } from './timing';
  import type { View } from './types';
  let { mapView, rate, view, width }: {mapView:MapView|null;rate:number;view:View;width:number} = $props();
  const measures = $derived(mapView?.measures ?? []);
  const span = $derived(view.end - view.start);
  const percent = (frame:number) => (frame/rate-view.start)/span*100;
  const boundaries = $derived([...new Set(measures.flatMap(m=>[m.start,m.end]))].filter(f=>f/rate>=view.start&&f/rate<=view.end));
  const labels = $derived.by(()=>{
    const points = measures.map(m=>({frame:m.start,label:m.label}));
    const first = measureAtFrame(measures, Math.round(view.start*rate));
    // Keep the containing measure legible when its start is offscreen.
    if(first && first.start/rate<view.start) points.push({frame:view.start*rate,label:first.label});
    return markersForView(points.sort((a,b)=>a.frame-b.frame),view.start*rate,view.end*rate,width,55).markers;
  });
  const timeTicks = $derived(Array.from({length:7},(_,i)=>view.start+span*i/6).filter(t=>
    !measureAtFrame(measures, Math.round(t*rate)) &&
    !boundaries.some(f=>Math.abs(f/rate-t)/span*width<55) &&
    !labels.some(p=>Math.abs(p.frame/rate-t)/span*width<55)));
</script>
<div class="practice-ruler" role="region" aria-label={measures.length?'Measure ruler':'Time ruler'}>
  {#each labels as label}
    <span class="measure-label" style:left={`${percent(label.frame)}%`} title={`Measure ${label.label}`}>{label.label}</span>
  {/each}
  {#each timeTicks as tick}
    <span class="time-label" style:left={`${(tick-view.start)/span*100}%`}>{time(tick)}</span>
  {/each}
</div>
<div class="wave-grid" aria-hidden="true">
  {#each boundaries as frame}<i class="measure-line" style:left={`${percent(frame)}%`}></i>{/each}
  {#each timeTicks as tick}<i class="time-line" style:left={`${(tick-view.start)/span*100}%`}></i>{/each}
</div>
<style>
  .practice-ruler{height:30px;position:absolute;inset:0 0 auto;font-size:11px;color:#bbcae2;font-variant-numeric:tabular-nums;pointer-events:none;overflow:hidden}
  .practice-ruler span{position:absolute;top:8px;white-space:nowrap}
  .measure-label{color:#c6dfd4;font-weight:600;padding-left:6px}
  .time-label{transform:translateX(-50%)}.time-label:first-child{transform:none;padding-left:8px}.time-label:last-child{transform:translateX(-100%);padding-right:8px}
  .wave-grid{position:absolute;inset:30px 0 0;pointer-events:none}.wave-grid i{position:absolute;height:100%}
  .measure-line{border-left:1px solid #a7c9bd88}.time-line{border-left:1px solid #bbccea0b}
</style>
