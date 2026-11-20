<script lang="ts">
  // The selected measure's properties panel (C4): meter, kind, how long a
  // pickup or incomplete measure lasts, grouping, the beat note, and "also the
  // following measures". Apply (Enter) is one direct edit with Undo; Cancel or
  // Esc closes without changes. A rejected edit keeps what was typed.
  import { untrack } from 'svelte';
  import { meterForm, readMeter } from './meter-form';
  import { KIND_NAMES } from './measure-tools';
  import { fractionText, parseFraction, type MapEdit, type MeasureKind, type MeasureView } from './timing';
  import type { ButtonId } from './actions';
  import type { Keymap } from './keymap';
  let { measure, error, keymap, available, run, oncancel }:
    { measure: MeasureView; error: string; keymap: Keymap; available: (id: ButtonId) => boolean; run: (id: ButtonId) => void; oncancel: () => void } = $props();
  const initial = untrack(() => measure);
  let form = $state(meterForm(initial.meter)), kind = $state<MeasureKind>(initial.kind), extent = $state(fractionText(initial.length)), following = $state(false);

  /** The edit the form describes; throws a readable message for invalid text. */
  export function edit(): MapEdit {
    return { type: 'set_notation', id: measure.id, meter: readMeter(form), kind, length: parseFraction(extent), following };
  }
</script>

<form class="properties" onsubmit={event => { event.preventDefault(); run('map_notation'); }}>
  <div class="row"><span class="name">Meter</span>
    <span class="meter" role="group" aria-label="Meter">
      <input aria-label="Meter numerator" type="number" min="1" max="99" step="1" bind:value={form.numerator}/>
      <span aria-hidden="true">/</span>
      <input aria-label="Meter denominator" title="1, 2, 4, 8, 16, 32 or 64" type="number" min="1" max="64" step="1" bind:value={form.denominator}/>
    </span>
  </div>
  <div class="row"><span class="name">Kind</span>
    <span class="segments" role="group" aria-label="Measure kind">
      {#each ['full', 'pickup', 'incomplete'] as const as value}<button type="button" aria-pressed={kind === value} onclick={() => { kind = value; }}>{KIND_NAMES[value]}</button>{/each}
    </span>
  </div>
  {#if kind !== 'full'}
    <label class="row"><span class="name">Lasts</span><input class="short" aria-label="Lasts (quarter notes)" bind:value={extent} placeholder="1 or 3/2"/><span class="hint">quarter notes</span></label>
  {/if}
  <label class="row"><span class="name">Grouping</span><input aria-label="Beat grouping" bind:value={form.grouping} placeholder="none, e.g. 2+2+3"/></label>
  <label class="row"><span class="name">Click on</span>
    <select aria-label="Beat note" bind:value={form.pulse}><option value="1">Quarter notes</option><option value="1/2">Eighth notes</option><option value="3/2">Dotted quarters</option><option value="2">Half notes</option><option value="1/4">Sixteenth notes</option></select>
  </label>
  <span class="rule"></span>
  <label class="check"><input type="checkbox" bind:checked={following}/>Also the following measures, until the next change</label>
  {#if measure.protected}<p class="hint" role="status">{measure.number === null ? 'The pickup' : `Measure ${measure.label}`} is locked. Unlock it to change its properties.</p>{/if}
  {#if error}<p class="error" role="alert">{error}</p>{/if}
  <div class="actions">
    <button type="button" class="compact-button" onclick={oncancel}>Cancel</button>
    <button type="submit" class="compact-button primary" disabled={!available('map_notation')} title={keymap.title('map_notation')}>{keymap.buttonLabel('map_notation', 'Apply')}</button>
  </div>
</form>

<style>
  .properties{display:flex;flex-direction:column;gap:12px}
  .row{display:flex;align-items:center;gap:8px;font-size:12px;color:var(--text-secondary)}
  .name{width:84px;flex:none;font-weight:600;color:var(--text)}
  .meter{display:inline-flex;align-items:center;gap:4px;color:var(--text-secondary);font-size:16px}
  input,select{height:30px;border:1px solid var(--border-strong);border-radius:6px;background:var(--input-bg);color:var(--text);font-size:13px;padding:0 8px}
  .meter input{width:42px;text-align:center;font-weight:700;appearance:textfield}
  .meter input::-webkit-inner-spin-button{appearance:none}
  .short{width:64px;text-align:right}
  label.row input:not(.short){flex:1;min-width:0}
  .segments{display:inline-flex;gap:2px;padding:2px;border:1px solid var(--border);border-radius:7px;background:var(--page)}
  .segments button{height:24px;padding:0 9px;border:0;border-radius:5px;background:transparent;color:var(--text-secondary);font-size:12px;font-weight:600}
  .segments button[aria-pressed=true]{background:var(--surface);color:var(--text);box-shadow:0 1px 2px #0000002e}
  .rule{height:1px;background:var(--border)}
  .check{display:flex;align-items:center;gap:6px;font-size:12px;color:var(--text-secondary)}
  .check input{height:auto}
  .hint{font-size:11px;line-height:1.45;color:var(--text-secondary)}
  .error{padding:7px 9px;border:1px solid var(--error-border);border-radius:6px;background:var(--error-bg);color:var(--error-text);font-size:12px;line-height:1.45}
  .actions{display:flex;justify-content:flex-end;gap:6px}
</style>
