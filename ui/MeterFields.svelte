<script lang="ts">
  import type {MeterForm} from './meter-form';
  let {value=$bindable(),prefix='',showMeter=true,showPulse=true,showGrouping=true,layout='stacked'}:{value:MeterForm;prefix?:string;showMeter?:boolean;showPulse?:boolean;showGrouping?:boolean;layout?:'stacked'|'compact'}=$props();
</script>
<div class:compact={layout==='compact'}>
{#if showMeter && layout==='compact'}
<div class="meter-field"><span>Meter</span><div class="meter-pair">
  <input aria-label={`${prefix}Meter numerator`} type="number" min="1" max="99" step="1" bind:value={value.numerator}/>
  <span class="slash" aria-hidden="true">/</span>
  <input aria-label={`${prefix}Meter denominator`} title="Use 1, 2, 4, 8, 16, 32 or 64" type="number" min="1" max="64" step="1" bind:value={value.denominator}/>
</div></div>
{:else if showMeter}<div class="meter-pair">
  <label>Meter numerator<input aria-label={`${prefix}Meter numerator`} type="number" min="1" max="99" step="1" bind:value={value.numerator}/></label>
  <label>Meter denominator<select aria-label={`${prefix}Meter denominator`} bind:value={value.denominator}>{#each [1,2,4,8,16,32,64] as d}<option value={d}>{d}</option>{/each}</select></label>
</div>
{/if}
{#if showPulse}<label>Pulse note value<select aria-label={`${prefix}Pulse note value`} bind:value={value.pulse}><option value="1">Quarter note</option><option value="1/2">Eighth note</option><option value="3/2">Dotted quarter</option><option value="2">Half note</option><option value="1/4">Sixteenth note</option></select></label>{/if}
{#if showGrouping}<label>Beat grouping (optional)<input aria-label={`${prefix}Beat grouping (optional)`} bind:value={value.grouping} placeholder="2+2+3"/></label>{/if}
</div>
<style>
  .compact{display:flex;flex-wrap:wrap;gap:12px;align-items:end}.compact label{margin-top:0;min-width:140px;max-width:220px;flex:1}.compact input,.compact select{padding:6px 7px}.meter-field{display:flex;flex-direction:column;gap:5px;font-size:10px;color:var(--text-secondary)}.compact .meter-pair{display:flex;gap:8px;align-items:center}.compact .meter-pair input{width:48px;height:36px;text-align:center;font-size:18px;padding:4px;appearance:textfield;font-variant-numeric:tabular-nums}.compact .meter-pair input::-webkit-inner-spin-button{appearance:none}.slash{font-size:27px;line-height:1;color:var(--muted)}
  .meter-pair{display:grid;grid-template-columns:1fr 1fr;gap:10px}label{display:flex;flex-direction:column;gap:5px;font-size:11px;margin-top:11px;color:var(--text-secondary)}input,select{min-width:0;width:100%;padding:7px 8px;border:1px solid var(--border-strong);border-radius:4px;background:var(--surface);color:var(--text);font:inherit;font-size:12px}
</style>
