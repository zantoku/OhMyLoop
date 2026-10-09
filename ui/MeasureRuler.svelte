<script lang="ts">
  import type { MapView } from './timing';
  import type { View } from './types';
  let { mapView, rate, view, onseek }:{mapView:MapView;rate:number;view:View;onseek:(seconds:number)=>void}=$props();
  const percent=(frame:number)=>(frame/rate-view.start)/(view.end-view.start)*100;
</script>
<div class="practice-measure-ruler" role="region" aria-label="Measure ruler">
  {#each mapView.measures.filter(m=>m.end/rate>view.start&&m.start/rate<view.end) as m(m.id)}
    <button style:left={`${Math.max(0,percent(m.start))}%`} style:width={`${Math.min(100,percent(m.end))-Math.max(0,percent(m.start))}%`} title={`Jump to measure ${m.label} (${m.meter.numerator}/${m.meter.denominator})`} onclick={()=>onseek(m.start/rate)}>{m.label}<span>{m.meter.numerator}/{m.meter.denominator}{m.kind==='incomplete'?' · partial':''}</span></button>
  {/each}
</div>
<style>
  .practice-measure-ruler{position:relative;height:25px;flex-shrink:0;background:#eff3f1;overflow:hidden;border-bottom:1px solid white}.practice-measure-ruler button{position:absolute;inset-block:0;text-align:left;overflow:hidden;white-space:nowrap;border:0;border-right:1px solid white;border-radius:0;background:#dce8e2;padding:0 8px;font-size:11px;color:#456154}.practice-measure-ruler span{margin-left:7px;font-size:9px;color:#68867a}
</style>
