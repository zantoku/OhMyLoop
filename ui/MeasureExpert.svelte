<script lang="ts">
  // The toolbar's ⋯ panel: expert controls next to the tools they belong to
  // (C2, C3, C6, C7). Exact barline positions and the chosen beat's position
  // apply when the field commits (Enter or leaving it), each as one edit; the
  // shift step, re-snapping and ε apply to the toolbar's Shift ◀ ▶. A rejected
  // edit keeps what was typed and says why.
  import { measureName } from './measure-tools';
  import { parseClock, time } from './model';
  import { parseFraction, pulseName, type MapEdit, type MeasureView } from './timing';
  import type { ButtonId } from './actions';
  import type { Keymap } from './keymap';
  let { measure, rate, beat = $bindable(null), step = $bindable(1), resnap = $bindable(true), epsilonMs = $bindable(30), detected, error, keymap, available, run }:
    { measure: MeasureView; rate: number; beat?: number | null; step?: number; resnap?: boolean; epsilonMs?: number; detected: boolean; error: string;
      keymap: Keymap; available: (id: ButtonId) => boolean; run: (id: ButtonId) => void } = $props();
  const beats = $derived(measure.grid.filter(p => p.quarter > 0));
  const chosen = $derived(beats.find(p => p.quarter === beat) ?? null);
  let before = $state(''), after = $state(''), at = $state(''), notated = $state(''), adding = $state(false), changed: 'start' | 'end' = 'start';
  // Show the map's positions whenever they change; a rejected edit leaves them, keeping the typed text.
  $effect(() => { before = time(measure.start / rate, 3); });
  $effect(() => { after = time(measure.end / rate, 3); });
  $effect(() => { at = chosen ? time(chosen.frame / rate, 3) : ''; notated = chosen ? quarters(chosen.quarter) : ''; if (chosen) adding = false; });
  /** A quarter-note offset as typed: `2` or `3/2`. */
  function quarters(value: number) {
    for (const den of [1, 2, 3, 4, 6, 8, 12, 16, 24, 32, 48, 64, 96, 960]) if (Math.abs(Math.round(value * den) - value * den) < 1e-6) return den === 1 ? String(Math.round(value)) : `${Math.round(value * den)}/${den}`;
    return String(value);
  }

  function frame(text: string) {
    const seconds = parseClock(text);
    if (!Number.isFinite(seconds)) throw new Error('Enter a time such as 1:23.104 or 83.104.');
    return Math.round(seconds * rate);
  }
  /** The barline move the last committed position field describes. */
  export function boundaryEdit(): MapEdit {
    return { type: 'move_boundary', id: changed === 'start' ? measure.start_id : measure.end_id, frame: frame(changed === 'start' ? before : after) };
  }
  /** True when the beat fields describe an anchor to set: a chosen beat, or a new one. */
  export function canBeat() { return !!chosen || adding; }
  /** Anchors the chosen (or a new) beat at the typed position and notated offset. */
  export function beatEdit(): MapEdit | null {
    if (!canBeat()) return null;
    const offset = parseFraction(notated);
    return { type: 'set_beat', measure: measure.id, id: chosen?.id ?? null, frame: frame(at), offset };
  }
  const commit = (which: 'start' | 'end') => () => { changed = which; run('map_boundary'); };
</script>

<div class="more">
  <h3 class="cap">Exact positions</h3>
  <label class="row"><span>Barline before</span><input aria-label={`Barline before ${measureName(measure)}`} value={before} oninput={e => { before = e.currentTarget.value; }} onchange={commit('start')}/></label>
  <label class="row"><span>Barline after</span><input aria-label={`Barline after ${measureName(measure)}`} value={after} oninput={e => { after = e.currentTarget.value; }} onchange={commit('end')}/></label>
  <span class="rule"></span>

  <h3 class="cap">Beats</h3>
  {#if beats.length}
    <div class="row">
      <select aria-label="Beat" value={adding ? 'new' : beat ?? ''} onchange={e => { const value = e.currentTarget.value; adding = value === 'new'; beat = value === '' || adding ? null : Number(value); }}>
        <option value="">Choose a beat</option>
        {#each beats as p (p.quarter)}<option value={p.quarter}>Beat {p.label}{p.id ? ' · anchored' : ''}</option>{/each}
        <option value="new">New anchor…</option>
      </select>
      <span>at</span><input class="time" aria-label="Beat position" disabled={!chosen && !adding} value={at} oninput={e => { at = e.currentTarget.value; }} onchange={() => run('map_beat')}/>
    </div>
    <label class="row"><span>Notated at</span><input class="count" aria-label="Notated offset (quarter notes)" disabled={!chosen && !adding} value={notated} placeholder="3/2"
      oninput={e => { notated = e.currentTarget.value; }} onchange={() => { if (at) run('map_beat'); }}/><span class="unit">quarter notes into the measure</span></label>
    <span class="hint">{adding ? 'Type where a beat missing from the grid falls, and its notated offset. ' : chosen ? `${chosen.id ? 'Anchored' : 'Follows the barlines'}. ` : ''}Drag the diamonds to move beats.</span>
    <div class="actions">
      <button class="compact-button" disabled={!available('map_anchor_playhead')} title={keymap.title('map_anchor_playhead')} onclick={() => run('map_anchor_playhead')}>{keymap.buttonLabel('map_anchor_playhead', 'Anchor at playhead')}</button>
      <button class="compact-button" disabled={!available('map_remove_beat')} title={keymap.title('map_remove_beat')} onclick={() => run('map_remove_beat')}>{keymap.buttonLabel('map_remove_beat', 'Remove anchor')}</button>
    </div>
  {:else}<span class="hint">This measure has no inner beats.</span>{/if}
  <span class="rule"></span>

  <h3 class="cap">Shift</h3>
  <label class="row"><span>Shift step</span><input class="count" aria-label="Shift step" type="number" min="1" max="64" step="1" bind:value={step}/><span class="unit">{pulseName(measure.meter.pulse)}-note {step === 1 ? 'beat' : 'beats'}</span></label>
  <div class="row">
    <label class="check" title={detected ? keymap.title('map_shift_resnap') : 'Analyze the song to re-snap to detected beats'}>
      <input type="checkbox" checked={resnap && detected} disabled={!available('map_shift_resnap')} onchange={() => run('map_shift_resnap')}/>{keymap.buttonLabel('map_shift_resnap', 'Re-snap to detected beats')}</label>
    {#if resnap && detected}<span>ε</span><input class="count" aria-label="Shift beat timing tolerance (ms)" type="number" min="0" max="1000" step="any" bind:value={epsilonMs}/><span>ms</span>{/if}
  </div>
  {#if !detected}<span class="hint">No detected beats: shifting keeps the authored beat timing.</span>{/if}
  <span class="rule"></span>
  <button class="compact-button ghost free" disabled={!available('map_remove')} title={keymap.title('map_remove')} onclick={() => run('map_remove')}>{keymap.buttonLabel('map_remove', 'Mark as free time')}</button>
  {#if error}<p class="error" role="alert">{error}</p>{/if}
</div>

<style>
  /* Fits the minimum window: the panel scrolls rather than leaving the window. */
  .more{display:flex;flex-direction:column;gap:7px;max-height:calc(100vh - 250px);margin:-4px -6px;padding:4px 6px;overflow:auto;overscroll-behavior:contain}
  .cap{font-size:11px;font-weight:700;letter-spacing:.06em;text-transform:uppercase;color:var(--text-secondary)}
  .row{display:flex;align-items:center;gap:8px;font-size:12px;color:var(--text-secondary)}
  label.row>span:first-child{flex:1}
  input,select{height:30px;border:1px solid var(--border-strong);border-radius:6px;background:var(--input-bg);color:var(--text);font-size:13px;padding:0 8px;font-variant-numeric:tabular-nums}
  label.row input:not(.count){width:104px;text-align:right}
  .time{width:96px;text-align:right}
  select{flex:1;min-width:0}
  .count{width:52px;text-align:center}
  .unit{white-space:nowrap}
  .check{flex:1;display:flex;align-items:center;gap:6px}
  .check input{height:auto}
  .hint{font-size:11px;line-height:1.45;color:var(--text-secondary)}
  .actions{display:flex;gap:6px}
  .rule{height:1px;background:var(--border)}
  .compact-button.free{align-self:flex-start;padding:0;color:var(--text-secondary)}
  .error{padding:7px 9px;border:1px solid var(--error-border);border-radius:6px;background:var(--error-bg);color:var(--error-text);font-size:12px;line-height:1.45}
</style>
