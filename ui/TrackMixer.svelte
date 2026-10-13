<script lang="ts">
 import {roleName,type StemTrack,type StemRole} from './stems';
 import type {Keymap} from './keymap';
 let {tracks,disabled=false,keymap,ongain,onmute,onsolo}:{tracks:StemTrack[];disabled?:boolean;keymap:Keymap;ongain:(role:StemRole,gain:number)=>void;onmute:(role:StemRole)=>void;onsolo:(role:StemRole)=>void}=$props();
 let drag:{role:StemRole;y:number;gain:number}|null=null;
 function gain(role:StemRole,value:number){if(!disabled)ongain(role,Math.round(Math.max(0,Math.min(1,value))*100)/100);}
 function key(event:KeyboardEvent,track:StemTrack){if(['ArrowUp','ArrowRight','ArrowDown','ArrowLeft','Home','End'].includes(event.key)){event.preventDefault();event.stopPropagation();gain(track.role,event.key==='Home'?0:event.key==='End'?1:track.gain+(['ArrowUp','ArrowRight'].includes(event.key)?.01:-.01));}}
</script>
<svelte:window onpointermove={event=>{if(drag)gain(drag.role,drag.gain+(drag.y-event.clientY)/100);}} onpointerup={()=>drag=null} onpointercancel={()=>drag=null}/>
<div class="track-mixer" role="group" aria-label="Stem mixer">
 {#each tracks as track (track.role)}
 <div class="track" class:muted={track.mute} class:soloed={track.solo}>
  <span class="name">{roleName(track.role)}</span>
  <div class="knob" role="slider" tabindex={disabled?-1:0} aria-disabled={disabled} aria-label={`${roleName(track.role)} gain`} aria-valuemin={0} aria-valuemax={100} aria-valuenow={Math.round(track.gain*100)} aria-valuetext={`${Math.round(track.gain*100)}%`} title="Drag vertically, scroll or use arrow keys. Double-click resets to 100%." style:--angle={`${-135+track.gain*270}deg`} style:--sweep={`${track.gain*270}deg`}
   onpointerdown={event=>{if(disabled)return;event.preventDefault();event.currentTarget.focus();event.currentTarget.setPointerCapture(event.pointerId);drag={role:track.role,y:event.clientY,gain:track.gain};}} onkeydown={event=>key(event,track)} onwheel={event=>{if(disabled)return;event.preventDefault();gain(track.role,track.gain+(event.deltaY<0?.02:-.02));}} ondblclick={()=>gain(track.role,1)}>
   <div class="dial"><span></span></div>
  </div>
  <span class="value">{Math.round(track.gain*100)}%</span>
  <div class="toggles"><button {disabled} class:enabled={track.mute} aria-pressed={track.mute} aria-label={`Mute ${roleName(track.role)}`} title={keymap.title(`mute_${track.role}`)} onclick={()=>onmute(track.role)}>M</button><button {disabled} class:enabled={track.solo} aria-pressed={track.solo} aria-label={`Solo ${roleName(track.role)}`} title={keymap.title(`solo_${track.role}`)} onclick={()=>onsolo(track.role)}>S</button></div>
 </div>
 {/each}
</div>
<style>
 .track-mixer{display:flex;justify-content:center;align-items:center;gap:26px;flex:1;min-width:0;flex-wrap:wrap;padding-inline:12px}.track{display:flex;flex-direction:column;align-items:center;gap:3px;min-width:44px}.name{font-size:10px;color:var(--text-secondary)}.knob{width:38px;height:38px;border-radius:50%;padding:4px;background:conic-gradient(from 225deg,var(--blue) 0deg,var(--blue) var(--sweep),var(--border) var(--sweep),var(--border) 270deg,transparent 270deg);cursor:ns-resize;touch-action:none}.dial{position:relative;border-radius:50%;background:var(--surface);height:100%;border:1px solid var(--border-strong);transform:rotate(var(--angle));box-shadow:0 1px 3px #0002}.dial span{position:absolute;top:3px;left:calc(50% - 1px);width:2px;height:8px;background:var(--blue);border-radius:1px}.value{font-size:9px;color:var(--muted);font-variant-numeric:tabular-nums}.toggles{display:flex;gap:3px}.toggles button{font-size:9px;min-width:20px;height:19px;border:1px solid var(--border);border-radius:3px;background:var(--sidebar);color:var(--muted);padding:0 4px}.toggles .enabled{background:var(--amber-bg);border-color:var(--amber-border);color:var(--amber-text)}.muted .knob,[aria-disabled=true]{opacity:.4}.soloed .name{color:var(--amber-text);font-weight:700}.knob:focus-visible{outline:3px solid var(--focus);outline-offset:3px}@media(max-width:700px){.track-mixer{gap:16px;flex-basis:100%;padding-top:8px;border-top:1px solid var(--border)}}
</style>
