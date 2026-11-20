<script lang="ts">
  // The passage at a glance: its extent and length, whether it loops, and
  // whether each repeat counts in. With a passage it opens exact A/B entry.
  import Popover from './Popover.svelte';
  import { range } from './model';
  import { passageSummary } from './practice-readout';
  import type { ButtonId } from './actions';
  import type { MeasureView } from './timing';
  import type { Loop } from './types';
  import type { Keymap } from './keymap';
  let { selection, looping, repeatCountIn, sectionName, measures, rate, duration, frozen, keymap, available, run, onrange }:
    { selection: Loop | null; looping: boolean; repeatCountIn: boolean; sectionName: string | null; measures: readonly MeasureView[]; rate: number; duration: number;
      frozen: boolean; keymap: Keymap; available: (id: ButtonId) => boolean; run: (id: ButtonId) => void; onrange: (loop: Loop) => void } = $props();
  const summary = $derived(selection ? passageSummary(selection, measures, rate) : null);
  const detail = $derived(summary ? [summary.length, sectionName, looping && repeatCountIn && measures.length ? 'count-in each repeat' : null].filter(Boolean).join(' · ') : '');
  const BOUNDS = [{ edge: 'start', name: 'A' }, { edge: 'end', name: 'B' }] as const;
  let open = $state(false), card = $state<HTMLButtonElement>();
  $effect(() => { if (!selection) open = false; });
  export function toggle() { if (selection) open = !open; }
  function typed(edge: 'start' | 'end', event: Event & { currentTarget: HTMLInputElement }) {
    if (!selection) return;
    const value = Number(event.currentTarget.value);
    const next = Number.isFinite(value) ? range(edge === 'start' ? value : selection.start, edge === 'end' ? value : selection.end, duration) : null;
    if (next) onrange(next); else event.currentTarget.value = selection[edge].toFixed(2);
  }
</script>

{#if summary && selection}
  <div class="anchor">
    <button class="loop-card" class:looping bind:this={card} aria-haspopup="dialog" aria-expanded={open} disabled={!available('passage')} title={keymap.title('passage')} onclick={() => run('passage')}>
      <span class="caption">{looping ? 'Looping' : 'Passage · loop off'}</span>
      <strong>{summary.range}</strong>
      <span class="detail">{detail}</span>
    </button>
    <Popover {open} label="Passage bounds" anchor={card} align="start" width={260} onclose={() => { open = false; }}>
      {#each BOUNDS as point (point.edge)}
        <div class="row">
          <label for={`passage-${point.edge}`}><b>{point.name}</b></label>
          <input id={`passage-${point.edge}`} aria-label={point.edge === 'start' ? 'Loop start seconds' : 'Loop end seconds'} type="number" step="any" disabled={frozen}
            min={point.edge === 'start' ? 0 : selection.start + .1} max={point.edge === 'start' ? selection.end - .1 : duration}
            value={selection[point.edge].toFixed(2)} onchange={event => typed(point.edge, event)}/>
          <span>seconds</span>
        </div>
      {/each}
      <span class="note">Exact bounds ignore snapping. Saved sections keep their own bounds.</span>
    </Popover>
  </div>
{:else}
  <div class="loop-card" aria-label="Passage">
    <span class="caption">No passage</span>
    <strong class="empty">Drag on the waveform</strong>
    <span class="detail">or press {keymap.shortcut('set_loop_start')} and {keymap.shortcut('set_loop_end')} at the playhead</span>
  </div>
{/if}

<style>
  .anchor{position:relative;min-width:0}
  .loop-card{display:flex;flex-direction:column;align-items:flex-start;gap:4px;min-width:0;padding:10px 14px;border:1px solid var(--border);border-radius:10px;background:transparent;color:var(--text-secondary);text-align:left}
  button.loop-card:hover:not(:disabled){background:var(--hover)}
  .loop-card.looping{border-color:var(--amber-border);background:var(--amber-bg);color:var(--amber-text)}
  .caption{font-size:11px;font-weight:700;letter-spacing:.06em;text-transform:uppercase}
  strong{font-size:22px;font-weight:700;color:var(--text);font-variant-numeric:tabular-nums;white-space:nowrap}
  strong.empty{font-size:16px;font-weight:600;color:var(--text-secondary)}
  .detail{max-width:100%;font-size:12px;white-space:nowrap;overflow:hidden;text-overflow:ellipsis}
  .row{display:flex;align-items:center;gap:8px;font-size:12px;color:var(--text-secondary)}
  .row label{width:20px;color:var(--amber-text)}
  input{width:90px;height:26px;padding:0;border:0;border-bottom:1px solid var(--border-strong);background:transparent;color:var(--text);font-size:16px;font-weight:700;text-align:right;font-variant-numeric:tabular-nums;appearance:textfield}
  input::-webkit-inner-spin-button,input::-webkit-outer-spin-button{appearance:none;margin:0}
  .note{font-size:11px;color:var(--text-secondary)}
</style>
