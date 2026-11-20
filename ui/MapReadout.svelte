<script lang="ts">
  // The Measure map's compact readout band (decision 3): the leading X back
  // to Practice (Foundations), the measure and beat
  // of the displayed map or draft, or the time in free time and without a
  // map; Undo and Redo; and, once a map exists, Redraft and the whole-song
  // settings. It gives the waveform the height.
  import Icon from './Icon.svelte';
  import { time } from './model';
  import { beatsInMeasure, musicalPoint } from './practice-readout';
  import { measureAtFrame, type MapView } from './timing';
  import type { ButtonId } from './actions';
  import type { Keymap } from './keymap';
  let { position, duration, rate, mapView, mapped, drafting, keymap, available, run }:
    { position: number; duration: number; rate: number; mapView: MapView | null; mapped: boolean; drafting: boolean;
      keymap: Keymap; available: (id: ButtonId) => boolean; run: (id: ButtonId) => void } = $props();
  const measures = $derived(mapView?.measures ?? []);
  const measure = $derived(measureAtFrame(measures, Math.round(position * rate)));
  const point = $derived(musicalPoint(measures, position, rate));
  const beats = $derived(beatsInMeasure(measure));
  const beat = $derived(point ? Math.max(1, Math.floor(point.beat + 1e-6)) : 0);
  const clock = $derived(`${time(position)} / ${time(duration, 0)}`);
</script>

<section class="readout" aria-label="Position">
  <button class="close" aria-label={keymap.buttonLabel('show_practice', 'Close measure map')} title={keymap.title('show_practice')} disabled={!available('show_practice')} onclick={() => run('show_practice')}><Icon name="close" size={16}/></button>
  {#if measure && point}
    <span class="cap">{measure.number === null ? 'Pickup' : 'Measure'}</span>
    {#if measure.number !== null}<strong>{point.label}</strong>{/if}
    {#if beats}<span class="beats" aria-label={`Beat ${beat} of ${beats}`}>{#each { length: beats } as _, index}<i class:on={index < beat}></i>{/each}</span>{/if}
    <span class="detail">beat {beat} · {measure.meter.numerator}/{measure.meter.denominator} · {measure.tempo_bpm.toFixed(1)} BPM · {clock}</span>
  {:else}
    <span class="cap">{measures.length ? 'Free time' : 'Time'}</span><strong>{time(position)}</strong>
    <span class="detail">/ {time(duration, 0)}{measures.length ? '' : ' · no measure map yet'}</span>
  {/if}
  <span class="spacer"></span>
  <button class="icon" aria-label={keymap.buttonLabel('map_undo', 'Undo')} disabled={!available('map_undo')} title={keymap.title('map_undo')} onclick={() => run('map_undo')}><Icon name="undo" size={16}/></button>
  <button class="icon" aria-label={keymap.buttonLabel('map_redo', 'Redo')} disabled={!available('map_redo')} title={keymap.title('map_redo')} onclick={() => run('map_redo')}><Icon name="redo" size={16}/></button>
  {#if mapped}
    <span class="sep"></span>
    <button class="compact-button" aria-pressed={drafting} aria-controls={drafting ? 'map-draft' : undefined} disabled={!available('map_redraft')} title={keymap.title('map_redraft')} onclick={() => run('map_redraft')}><Icon name="wand" size={15}/>{keymap.buttonLabel('map_redraft', 'Redraft')}</button>
    <button class="compact-button" aria-haspopup="dialog" disabled={!available('map_song_settings')} title={keymap.title('map_song_settings')} onclick={() => run('map_song_settings')}><Icon name="sliders" size={15}/>{keymap.buttonLabel('map_song_settings', 'Song settings')}</button>
  {/if}
</section>

<style>
  .readout{flex:none;height:54px;display:flex;align-items:center;gap:12px;padding:0 24px 0 16px;white-space:nowrap}
  .close{width:30px;height:30px;display:grid;place-items:center;padding:0;border:0;border-radius:6px;background:transparent;color:var(--text-secondary)}
  .close:hover:not(:disabled){background:var(--hover);color:var(--text)}
  .cap{font-size:11px;font-weight:700;letter-spacing:.06em;text-transform:uppercase;color:var(--text-secondary)}
  strong{font-size:30px;line-height:1;font-weight:700;font-variant-numeric:tabular-nums}
  .beats{display:flex;gap:4px}
  .beats i{width:12px;height:12px;border-radius:3px;background:var(--border-strong)}
  .beats i.on{background:var(--blue)}
  .detail{font-size:13px;color:var(--text-secondary);font-variant-numeric:tabular-nums;overflow:hidden;text-overflow:ellipsis}
  .spacer{flex:1}
  .icon{width:30px;height:30px;display:grid;place-items:center;padding:0;border:1px solid var(--border);border-radius:6px;background:var(--surface);color:var(--text-secondary)}
  .icon:hover:not(:disabled){background:var(--hover);color:var(--text)}
  .sep{width:1px;align-self:stretch;margin:12px 4px;background:var(--border)}
  .compact-button[aria-pressed=true]{background:var(--selected);border-color:var(--selected-border)}
</style>
