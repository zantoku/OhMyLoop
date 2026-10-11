<script lang="ts">
  import type { ButtonId } from './actions';
  import type { Keymap } from './keymap';
  let { enabled, level=$bindable(50), keymap, button, available, run, native }:
    {enabled:boolean;level?:number;keymap:Keymap;button:ButtonId;available:boolean;run:()=>void;native:boolean}=$props();
</script>
<div class="metronome-controls" role="group" aria-label="Metronome controls">
  <button class="button" aria-pressed={enabled} disabled={!available} title={keymap.title(button)} onclick={run}>{keymap.buttonLabel(button,enabled?'Metronome on':'Metronome off')}</button>
  <label>Click level<input aria-label="Metronome volume" type="range" min="0" max="100" bind:value={level} disabled={!native}/><span>{level}%</span></label>
</div>
<style>
  .metronome-controls{display:flex;align-items:center;gap:12px;flex-wrap:wrap;font-size:11px;color:var(--muted)}
  .button{font-size:11px;padding:7px 10px}input{accent-color:var(--blue)}
  label{display:flex;align-items:center;gap:7px}input{width:80px}label span{min-width:30px;font-variant-numeric:tabular-nums}
</style>
