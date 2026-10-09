<script lang="ts">
  import type {MeterForm} from './meter-form';
  let {value=$bindable(),prefix='',showGrouping=true,layout='stacked'}:{value:MeterForm;prefix?:string;showGrouping?:boolean;layout?:'stacked'|'compact'}=$props();
</script>
<div class:compact={layout==='compact'}>
<div class="meter-pair">
  <label>Meter numerator<input aria-label={`${prefix}Meter numerator`} type="number" min="1" max="99" step="1" bind:value={value.numerator}/></label>
  <label>Meter denominator<select aria-label={`${prefix}Meter denominator`} bind:value={value.denominator}>{#each [1,2,4,8,16,32,64] as d}<option value={d}>{d}</option>{/each}</select></label>
</div>
<label>Pulse note value<select aria-label={`${prefix}Pulse note value`} bind:value={value.pulse}><option value="1">Quarter note</option><option value="1/2">Eighth note</option><option value="3/2">Dotted quarter</option><option value="2">Half note</option><option value="1/4">Sixteenth note</option></select></label>
{#if showGrouping}<label>Grouping (optional)<input aria-label={`${prefix}Grouping (optional)`} bind:value={value.grouping} placeholder="2+2+3"/></label>{/if}
</div>
<style>
  .compact{display:grid;grid-template-columns:1.2fr 1fr 1fr;gap:10px;align-items:end}.compact .meter-pair{gap:8px}.compact label{margin-top:0}.compact input,.compact select{padding:6px 7px}.compact .meter-pair label{min-width:0}.compact label{min-width:0}
  .meter-pair{display:grid;grid-template-columns:1fr 1fr;gap:10px}label{display:flex;flex-direction:column;gap:5px;font-size:11px;margin-top:11px;color:#596b86}input,select{min-width:0;width:100%;padding:7px 8px;border:1px solid #d3ddeb;border-radius:4px;background:#fff;color:#334966;font:inherit;font-size:12px}
</style>
