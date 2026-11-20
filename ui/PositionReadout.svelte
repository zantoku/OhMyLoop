<script lang="ts">
  // Where am I: measure and beat, readable at a distance, or the time in a
  // song without a measure map. The measure opens "Go to measure".
  import Popover from './Popover.svelte';
  import { time } from './model';
  import { beatsInMeasure } from './practice-readout';
  import { measureAtFrame, type MapView, type MusicalPosition } from './timing';
  import type { ButtonId } from './actions';
  import type { Keymap } from './keymap';
  let { position, duration, rate, mapView, musical, countingIn = false, measureNumber = $bindable(1), keymap, available, run }:
    { position: number; duration: number; rate: number; mapView: MapView | null; musical: MusicalPosition | null | undefined; countingIn?: boolean; measureNumber?: number;
      keymap: Keymap; available: (id: ButtonId) => boolean; run: (id: ButtonId) => void } = $props();
  const mapped = $derived(!!mapView?.measures.length);
  const measure = $derived(mapped ? measureAtFrame(mapView!.measures, Math.round(position * rate)) : null);
  const beats = $derived(beatsInMeasure(measure));
  const beat = $derived(musical ? Math.max(1, Math.floor(musical.beat + 1e-6)) : 0);
  let gotoOpen = $state(false), trigger = $state<HTMLButtonElement>();
  export function toggleGoto() { gotoOpen = !gotoOpen; }
  function go(event: SubmitEvent) { event.preventDefault(); run('goto_measure'); gotoOpen = false; }
</script>

<div class="position" aria-label="Position">
  {#if mapped}
    <div class="anchor">
      <button class="big" bind:this={trigger} aria-haspopup="dialog" aria-expanded={gotoOpen} disabled={!available('measure_readout')} title={keymap.title('measure_readout')} onclick={() => run('measure_readout')}>
        <span class="caption">{musical ? 'Measure' : 'Free time'}</span><strong class:measure={!!musical} class:long={(musical?.label.length ?? 0) > 3}>{musical?.label ?? time(position)}</strong>
      </button>
      <Popover open={gotoOpen} label="Go to measure" anchor={trigger} align="start" width={240} onclose={() => { gotoOpen = false; }}>
        <form class="goto" onsubmit={go}>
          <label for="goto-measure">Go to measure</label>
          <input id="goto-measure" aria-label="Go to measure number" type="number" min="1" step="1" bind:value={measureNumber}/>
          <button class="go" type="submit" disabled={!available('goto_measure')} title={keymap.title('goto_measure')}>{keymap.buttonLabel('goto_measure', 'Go')}</button>
        </form>
      </Popover>
    </div>
    <div class="detail">
      {#if countingIn}<span class="counting" role="status">Counting in…</span>
      {:else if musical && beats}<div class="beats" aria-label={`Beat ${beat} of ${beats}`}>{#each { length: beats } as _, index}<span class:on={index < beat}></span>{/each}</div>{/if}
      <span>{musical ? `beat ${beat} · ` : ''}{time(position)} / {time(duration, 0)}</span>
    </div>
  {:else}
    <div class="big"><span class="caption">Position</span><strong>{time(position)}</strong></div>
    <div class="detail">
      <span>of {time(duration, 0)}</span>
      <button class="link" disabled={!available('show_measure_map')} title={keymap.title('show_measure_map')} onclick={() => run('show_measure_map')}>Add a measure map ›</button>
    </div>
  {/if}
</div>

<style>
  .position{display:flex;align-items:flex-end;gap:12px;min-width:0}
  .anchor{position:relative}
  .big{display:flex;flex-direction:column;align-items:flex-start;padding:0;border:0;background:transparent;color:var(--text);text-align:left}
  button.big{border-radius:6px}
  button.big:hover:not(:disabled) .caption{color:var(--text)}
  .caption{font-size:11px;font-weight:700;letter-spacing:.06em;text-transform:uppercase;color:var(--text-secondary)}
  strong{font-size:64px;line-height:.95;font-weight:700;letter-spacing:-.02em;font-variant-numeric:tabular-nums;white-space:nowrap}
  strong.measure{font-size:76px;line-height:.9;min-width:1.2ch}
  strong.measure.long{font-size:52px}
  .detail{display:flex;flex-direction:column;gap:6px;padding-bottom:5px;font-size:13px;color:var(--text-secondary);font-variant-numeric:tabular-nums;white-space:nowrap}
  .beats{display:flex;gap:5px}
  .counting{font-weight:700;color:var(--amber-text)}
  .beats span{width:18px;height:18px;border-radius:4px;background:var(--border-strong)}
  .beats span.on{background:var(--blue)}
  .link{align-self:flex-start;border:0;padding:0;background:transparent;color:var(--link);font-size:12px;font-weight:600}
  .link:hover:not(:disabled){text-decoration:underline}
  .goto{display:flex;align-items:center;gap:8px;font-size:12px;font-weight:600}
  .goto label{flex:1}
  .goto input{width:64px;height:28px;border:0;border-bottom:1px solid var(--border-strong);background:transparent;color:var(--text);font-size:16px;font-weight:700;text-align:right}
  .go{height:28px;padding:0 12px;border:1px solid var(--border-strong);border-radius:6px;background:var(--surface);font-size:12px;font-weight:600}
  .go:hover:not(:disabled){background:var(--hover)}
</style>
