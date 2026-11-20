<script lang="ts">
  // Counting a free-time (uncounted) range as one measure (C7). The measure
  // spans the whole range with evenly spaced beats; the measures around it
  // stay where they are. One direct edit with Undo; a rejection keeps the form.
  import { untrack } from 'svelte';
  import { meterForm, readMeter } from './meter-form';
  import { time } from './model';
  import type { UnmappedRange } from './map-ranges';
  import type { Meter } from './timing';
  import type { ButtonId } from './actions';
  import type { Keymap } from './keymap';
  let { range, rate, meter, error, keymap, available, run, oncancel }:
    { range: UnmappedRange; rate: number; meter: Meter | null; error: string; keymap: Keymap; available: (id: ButtonId) => boolean; run: (id: ButtonId) => void; oncancel: () => void } = $props();
  let form = $state(meterForm(untrack(() => meter ? { ...meter, grouping: [] } : undefined)));
  /** The meter for the new measure; throws a readable message for invalid text. */
  export function chosenMeter(): Meter { return readMeter(form); }
</script>

<form class="count" onsubmit={event => { event.preventDefault(); run('map_reclaim'); }}>
  <p class="hint">{time(range.start / rate, 3)} – {time(range.end / rate, 3)} becomes one measure with evenly spaced beats. The measures around it stay where they are.</p>
  <div class="row"><span class="name">Meter</span>
    <span class="meter" role="group" aria-label="Meter">
      <input aria-label="Meter numerator" type="number" min="1" max="99" step="1" bind:value={form.numerator}/><span aria-hidden="true">/</span>
      <input aria-label="Meter denominator" type="number" min="1" max="64" step="1" bind:value={form.denominator}/>
    </span>
  </div>
  <label class="row"><span class="name">Click on</span>
    <select aria-label="Beat note" bind:value={form.pulse}><option value="1">Quarter notes</option><option value="1/2">Eighth notes</option><option value="3/2">Dotted quarters</option><option value="2">Half notes</option><option value="1/4">Sixteenth notes</option></select>
  </label>
  {#if error}<p class="error" role="alert">{error}</p>{/if}
  <div class="actions">
    <button type="button" class="compact-button" onclick={oncancel}>Cancel</button>
    <button type="submit" class="compact-button primary" disabled={!available('map_reclaim')} title={keymap.title('map_reclaim')}>{keymap.buttonLabel('map_reclaim', 'Create measure')}</button>
  </div>
</form>

<style>
  .count{display:flex;flex-direction:column;gap:12px}
  .row{display:flex;align-items:center;gap:8px;font-size:12px}
  .name{width:70px;flex:none;font-weight:600}
  .meter{display:inline-flex;align-items:center;gap:4px;color:var(--text-secondary);font-size:16px}
  input,select{height:30px;border:1px solid var(--border-strong);border-radius:6px;background:var(--input-bg);color:var(--text);font-size:13px;padding:0 8px}
  .meter input{width:42px;text-align:center;font-weight:700;appearance:textfield}
  .meter input::-webkit-inner-spin-button{appearance:none}
  select{flex:1}
  .hint{font-size:11px;line-height:1.45;color:var(--text-secondary)}
  .error{padding:7px 9px;border:1px solid var(--error-border);border-radius:6px;background:var(--error-bg);color:var(--error-text);font-size:12px;line-height:1.45}
  .actions{display:flex;justify-content:flex-end;gap:6px}
</style>
