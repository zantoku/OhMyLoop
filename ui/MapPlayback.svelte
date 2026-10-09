<script lang="ts">
  import Icon from './Icon.svelte';
  import {time} from './model';
  import type {Keymap} from './keymap';
  import type {ButtonId} from './actions';
  import type {Transport} from './types';
  let {transport,duration,speed,loopEnabled,keymap,available,run,onspeed,disabled=false}:
    {transport:Transport;duration:number;speed:number;loopEnabled:boolean;keymap:Keymap;available:(id:ButtonId)=>boolean;run:(id:ButtonId)=>void;onspeed:(speed:number)=>void;disabled?:boolean}=$props();
</script>
<div class="map-playback" role="group" aria-label="Measure mapping playback">
  <button class="map-play" aria-label={keymap.buttonLabel('play',transport.playing?'Pause':'Play')} disabled={!available('play')} title={keymap.title('play')} onclick={()=>run('play')}><Icon name={transport.playing?'pause':'play'} size={19}/></button>
  <span class="position"><strong>{time(transport.position,2)}</strong><span>/ {time(duration,0)}</span></span>
  <label class="speed">Speed <input aria-label="Playback speed" type="range" min="25" max="200" step="1" value={speed*100} {disabled} oninput={e=>onspeed(Number(e.currentTarget.value)/100)}/></label>
  <label class="speed-number"><input aria-label="Speed percent" type="number" min="25" max="200" step="1" value={Math.round(speed*100)} {disabled} onchange={e=>onspeed(Number(e.currentTarget.value)/100)}/>%</label>
  {#if loopEnabled}<button class="loop-off" disabled={!available('mapping_loop_off')} title={keymap.title('mapping_loop_off')} onclick={()=>run('mapping_loop_off')}>{keymap.buttonLabel('mapping_loop_off','Turn looping off')}</button>{/if}
</div>
<style>
  .map-playback{display:flex;align-items:center;gap:12px;flex-wrap:wrap;padding:8px 10px;color:#d9e4f5;font-size:11px;background:#17273bed;border:1px solid #ffffff24;border-radius:6px;box-shadow:0 3px 12px #13223830;cursor:default}
  .map-play{display:grid;place-items:center;width:32px;height:32px;border:0;border-radius:50%;color:#213954;background:#d7e5ef;flex-shrink:0;padding:0}.map-play:hover{background:#fff}.map-play:disabled{opacity:.45}
  .position{display:flex;gap:5px;align-items:baseline;font-variant-numeric:tabular-nums}.position>span{color:#99aac3}.position strong{font-weight:500;min-width:53px}
  .speed{display:flex;align-items:center;gap:8px}.speed input{width:95px;accent-color:#b4c9df;height:20px}.speed-number{display:flex;gap:4px;align-items:center}.speed-number input{width:49px;color:#e1ebfa;background:#ffffff0c;border:1px solid #ffffff26;border-radius:3px;padding:4px;font:inherit;font-variant-numeric:tabular-nums;appearance:textfield}.speed-number input::-webkit-inner-spin-button{appearance:none}
  .loop-off{background:none;border:0;color:#d9e4f5;font:inherit;padding:5px;text-decoration:underline;text-underline-offset:3px}
  button:focus-visible,input:focus-visible{outline:2px solid #cad9f0;outline-offset:2px}
  @media(max-width:1000px){.position>span{display:none}.map-playback{gap:8px}.speed input{width:70px}}
</style>
