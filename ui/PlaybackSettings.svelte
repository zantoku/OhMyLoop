<script lang="ts">
 import Icon from './Icon.svelte';
 import type {Keymap} from './keymap';
 import type {ButtonId} from './actions';
 import type {Controls} from './types';
 import {SEMITONES, SPEED_PERCENT, speedPercent, TUNING_CENTS} from './playback-controls';
 let {controls,keymap,disabled=false,onchange,runButton}:{controls:Controls;disabled?:boolean;keymap:Keymap;onchange:(change:Partial<Controls>)=>void;runButton:(id:ButtonId)=>void}=$props();
 const slower=[{id:'speed_down_10',label:'−10',name:'Slower by 10 percent'},{id:'speed_down_5',label:'−5',name:'Slower by 5 percent'}] as const;
 const faster=[{id:'speed_up_5',label:'+5',name:'Faster by 5 percent'},{id:'speed_up_10',label:'+10',name:'Faster by 10 percent'}] as const;
 /** Applies a typed value, or restores the shown value when the entry is not a number. */
 function entered(event:Event&{currentTarget:HTMLInputElement},shown:number,apply:(value:number)=>void){
  const value=event.currentTarget.value.trim()===''?NaN:Number(event.currentTarget.value);
  if(Number.isFinite(value))apply(value);else event.currentTarget.value=String(shown);
 }
 const cents=$derived(Math.round(controls.tuning_cents*10)/10);
</script>
<div class="compact-settings">
 <section aria-label="Playback speed"><label for="speed-percent">Speed</label>
  <div class="stepper">
   {#each slower as step}<button {disabled} class="step" aria-label={step.name} title={keymap.title(step.id)} onclick={()=>runButton(step.id)}>{step.label}</button>{/each}
   <div class="number"><input id="speed-percent" {disabled} aria-label="Speed percent" type="number" min={SPEED_PERCENT.min} max={SPEED_PERCENT.max} step="any" value={speedPercent(controls.speed)} onchange={event=>entered(event,speedPercent(controls.speed),value=>onchange({speed:value/100}))}/><span>%</span></div>
   {#each faster as step}<button {disabled} class="step" aria-label={step.name} title={keymap.title(step.id)} onclick={()=>runButton(step.id)}>{step.label}</button>{/each}
  </div>
  <button {disabled} class="icon-button" aria-label="Reset speed" title={keymap.title('reset_speed')} onclick={()=>runButton('reset_speed')}><Icon name="reset" size={15}/></button>
 </section>
 <section aria-label="Playback pitch"><label for="pitch-semitones">Pitch</label>
  <div class="stepper">
   <button {disabled} class="step" aria-label="Transpose down" title={keymap.title('transpose_down')} onclick={()=>runButton('transpose_down')}>−</button>
   <div class="number"><input id="pitch-semitones" {disabled} aria-label="Pitch semitones" type="number" min={SEMITONES.min} max={SEMITONES.max} step="1" value={controls.semitones} onchange={event=>entered(event,controls.semitones,value=>onchange({semitones:Math.round(value)}))}/><span>st</span></div>
   <button {disabled} class="step" aria-label="Transpose up" title={keymap.title('transpose_up')} onclick={()=>runButton('transpose_up')}>+</button>
  </div>
  <button {disabled} class="icon-button" aria-label="Reset pitch" title={keymap.title('reset_pitch')} onclick={()=>runButton('reset_pitch')}><Icon name="reset" size={15}/></button>
 </section>
 <section aria-label="Tuning"><label for="tuning-cents" title="Fine offset for recordings tuned away from A = 440 Hz">Tuning</label>
  <div class="number"><input id="tuning-cents" {disabled} aria-label="Tuning cents" type="number" min={TUNING_CENTS.min} max={TUNING_CENTS.max} step="0.1" value={cents} onchange={event=>entered(event,cents,value=>onchange({tuning_cents:value}))}/><span>¢</span></div>
  <button {disabled} class="icon-button" aria-label="Reset tuning" title={keymap.title('reset_tuning')} onclick={()=>runButton('reset_tuning')}><Icon name="reset" size={15}/></button>
 </section>
</div>
<style>
 .compact-settings{display:flex;flex-wrap:wrap;gap:10px 28px;padding:10px 0 0;flex-shrink:0}.compact-settings section{display:flex;align-items:center;gap:8px;font-size:11px;min-width:0}.stepper{display:flex;align-items:center;gap:3px}.step{min-width:30px;height:26px;padding:0 6px;border:1px solid var(--border);border-radius:4px;background:var(--surface);color:var(--text-secondary);font-size:11px;font-variant-numeric:tabular-nums}.step:hover:not(:disabled){background:var(--hover);color:var(--text)}.number{display:flex;align-items:center;gap:2px;margin-inline:4px}.number input{width:52px;border:0;border-bottom:1px solid var(--border);background:transparent;color:var(--text);font:inherit;font-weight:500;padding:3px 0;text-align:right;font-variant-numeric:tabular-nums;appearance:textfield}.number input::-webkit-inner-spin-button,.number input::-webkit-outer-spin-button{appearance:none;margin:0}@media(max-width:650px){.compact-settings{flex-direction:column;align-items:flex-start;gap:8px}}
</style>
