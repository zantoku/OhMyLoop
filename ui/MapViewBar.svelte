<script lang="ts">
  // The bar below the Measure map's waveform (decision 7): the detected-beats
  // toggle and a legend, a preview's Apply and Discard when the drafting
  // drawer isn't showing them (numbering and grouping previews), and the view
  // commands Fit measure and Whole song.
  import Icon from './Icon.svelte';
  import type { ButtonId } from './actions';
  import type { Keymap } from './keymap';
  let { detected, hasDetected, preview, note = null, keymap, available, run }:
    { detected: boolean; hasDetected: boolean; preview: { warnings: string[] } | null; note?: string | null; keymap: Keymap; available: (id: ButtonId) => boolean; run: (id: ButtonId) => void } = $props();
</script>

<div class="view-bar" role="group" aria-label="View">
  <button class="detected" aria-pressed={detected && hasDetected} aria-disabled={!hasDetected} title={hasDetected ? keymap.title('map_show_detected') : 'Analyze the song to see detected beats'}
    onclick={() => { if (available('map_show_detected')) run('map_show_detected'); }}><Icon name="detected" size={16}/>{keymap.buttonLabel('map_show_detected', 'Detected beats')}</button>
  {#if preview}
    <div class="preview" role="status">
      <span><b>Preview</b> · shown in green; your map is unchanged.{#each preview.warnings as warning} {warning}{/each}</span>
      <button class="compact-button" disabled={!available('map_discard')} title={keymap.title('map_discard')} onclick={() => run('map_discard')}>{keymap.buttonLabel('map_discard', 'Discard')}</button>
      <button class="compact-button primary" disabled={!available('map_apply')} title={keymap.title('map_apply')} onclick={() => run('map_apply')}>{keymap.buttonLabel('map_apply', 'Apply map')}</button>
    </div>
  {:else}
    <span class="legend" aria-hidden="true"><span><i class="map"></i>Map</span><span><i class="mark"></i>Detected</span><span><i class="draft"></i>Draft</span></span>
  {/if}
  {#if note}<span class="note" role="status">{note} · <button class="link" disabled={!available('map_redraft')} title={keymap.title('map_redraft')} onclick={() => run('map_redraft')}>Redraft…</button></span>{/if}
  <span class="spacer"></span>
  <button class="compact-button ghost" disabled={!available('map_fit')} title={keymap.title('map_fit')} onclick={() => run('map_fit')}>{keymap.buttonLabel('map_fit', 'Fit measure')}</button>
  <button class="compact-button ghost" disabled={!available('map_whole')} title={keymap.title('map_whole')} onclick={() => run('map_whole')}>{keymap.buttonLabel('map_whole', 'Whole song')}</button>
</div>

<style>
  .view-bar{flex:none;display:flex;align-items:center;gap:10px;min-height:34px}
  .detected{height:30px;display:flex;align-items:center;gap:7px;padding:0 10px;border:1px solid var(--border);border-radius:6px;background:var(--surface);color:var(--text-secondary);font-size:12px;font-weight:600}
  .detected:hover:not([aria-disabled=true]){background:var(--hover)}
  .detected[aria-pressed=true]{border-color:var(--amber-border);background:var(--amber-bg);color:var(--amber-text)}
  .detected[aria-disabled=true]{opacity:.45}
  .legend{display:flex;align-items:center;gap:12px;font-size:11px;color:var(--text-secondary)}
  .legend span{display:inline-flex;align-items:center;gap:5px}
  .legend i{display:inline-block;width:2px;height:12px}
  .legend .map{background:var(--text-secondary)}
  .legend .mark{width:0;height:0;border-inline:4px solid transparent;border-bottom:8px solid var(--amber-text)}
  .legend .draft{background-image:repeating-linear-gradient(180deg,#5fb88a 0 3px,transparent 3px 5px)}
  .preview{display:flex;align-items:center;gap:8px;padding:2px 2px 2px 10px;border:1px solid var(--success-border);border-radius:8px;background:var(--success-bg);color:var(--success-text);font-size:12px}
  .spacer{flex:1}
  .note{display:flex;align-items:center;gap:4px;font-size:11px;color:var(--amber-text)}
  .link{border:0;padding:0;background:transparent;color:inherit;font:inherit;font-weight:600;text-decoration:underline;text-underline-offset:2px}
</style>
