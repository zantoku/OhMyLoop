<script lang="ts">
  // Below the waveform: the sections toggle, snapping, and the passage's
  // A and B bounds (measure · beat, or times without a measure), plus Fit,
  // Save as section and Clear. Exact seconds are entered from the Loop card.
  import Icon from './Icon.svelte';
  import { time } from './model';
  import { boundText } from './practice-readout';
  import type { ButtonId } from './actions';
  import type { Keymap } from './keymap';
  import type { MeasureView } from './timing';
  import type { Loop } from './types';
  let { sectionsId, sectionsOpen, selection, snapToMeasures, measures, rate, keymap, available, run }:
    { sectionsId: string; sectionsOpen: boolean; selection: Loop | null; snapToMeasures: boolean; measures: readonly MeasureView[]; rate: number;
      keymap: Keymap; available: (id: ButtonId) => boolean; run: (id: ButtonId) => void } = $props();
  const BOUNDS = [{ edge: 'start', name: 'A', button: 'jump_loop_start' }, { edge: 'end', name: 'B', button: 'jump_loop_end' }] as const;
  const mapped = $derived(measures.length > 0);
</script>

<div class="passage-bar" aria-label="Passage" role="group">
  <button class="toggle" aria-controls={sectionsId} aria-expanded={sectionsOpen} disabled={!available('sections')} title={keymap.title('sections')} onclick={() => run('sections')}>
    <Icon name="sidebar" size={15}/>{keymap.buttonLabel('sections', 'Sections')}
  </button>
  {#if mapped}
    <span class="divider" aria-hidden="true"></span>
    <button class="snap" aria-label={keymap.buttonLabel('loop_snap', 'Snap to measures')} aria-pressed={snapToMeasures} disabled={!available('loop_snap')} title={keymap.title('loop_snap')} onclick={() => run('loop_snap')}><Icon name="magnet" size={16}/></button>
  {/if}
  <span class="divider" aria-hidden="true"></span>
  {#if selection}
    {#each BOUNDS as point (point.edge)}
      {@const at = boundText(measures, selection[point.edge], rate) ?? time(selection[point.edge])}
      {#if point.edge === 'end'}<span class="arrow" aria-hidden="true">→</span>{/if}
      <button class="bound" aria-label={keymap.description(keymap.actionFor(point.button))} disabled={!available(point.button)} title={keymap.title(point.button)} onclick={() => run(point.button)}>
        <b>{point.name}</b><span>{at}</span>
      </button>
    {/each}
    <span class="spacer"></span>
    <button class="text" disabled={!available('fit_loop')} title={keymap.title('fit_loop')} onclick={() => run('fit_loop')}>{keymap.buttonLabel('fit_loop', 'Fit passage')}</button>
    <button class="text" disabled={!available('save_selection')} title={keymap.title('save_selection')} onclick={() => run('save_selection')}>{keymap.buttonLabel('save_selection', 'Save as section')}</button>
    <button class="clear" aria-label={keymap.buttonLabel('clear_loop', 'Clear passage')} disabled={!available('clear_loop')} title={keymap.title('clear_loop')} onclick={() => run('clear_loop')}><Icon name="close" size={16}/></button>
  {:else}
    <span class="hint">Drag across the waveform to choose a passage{mapped && snapToMeasures ? ' of whole measures' : ''}, or press <kbd>{keymap.shortcut('set_loop_start')}</kbd> and <kbd>{keymap.shortcut('set_loop_end')}</kbd> at the playhead.</span>
  {/if}
</div>

<style>
  .passage-bar{flex:none;display:flex;align-items:center;gap:10px;min-height:34px;white-space:nowrap}
  .toggle{height:30px;display:flex;align-items:center;gap:7px;padding:0 10px;border:1px solid var(--border);border-radius:6px;background:var(--surface);color:var(--text-secondary);font-size:12px}
  .toggle[aria-expanded=true]{border-color:var(--border-strong);background:var(--selected)}
  .snap{display:grid;place-items:center;width:30px;height:30px;padding:0;border:1px solid var(--border);border-radius:6px;background:var(--surface);color:var(--text-secondary)}
  .snap[aria-pressed=true]{border-color:var(--border-strong);background:var(--selected);color:var(--blue)}
  .toggle:hover:not(:disabled),.snap:hover:not(:disabled){background:var(--hover)}
  .divider{width:1px;height:20px;background:var(--border)}
  .bound{height:30px;display:flex;align-items:center;gap:8px;padding:0 10px;border:1px solid var(--amber-border);border-radius:6px;background:var(--amber-bg);color:var(--amber-text)}
  .bound:hover:not(:disabled){background:var(--hover)}
  .bound span{color:var(--text);font-variant-numeric:tabular-nums}
  .arrow{color:var(--text-secondary)}
  .spacer{flex:1}
  .text{height:30px;padding:0 8px;border:0;border-radius:5px;background:transparent;color:var(--link);font-weight:600}
  .clear{display:grid;place-items:center;width:30px;height:30px;padding:0;border:0;border-radius:5px;background:transparent;color:var(--text-secondary)}
  .text:hover:not(:disabled),.clear:hover:not(:disabled){background:var(--hover)}
  .hint{font-size:12px;color:var(--text-secondary);overflow:hidden;text-overflow:ellipsis}
  kbd{display:inline-block;min-width:18px;padding:0 4px;border:1px solid var(--border-strong);border-radius:3px;font:inherit;text-align:center}
</style>
