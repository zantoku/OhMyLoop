<script lang="ts">
  // The Practice mix: the metronome click first, then the stems. Without
  // stems it points to the Stems view instead.
  import Knob from './Knob.svelte';
  import TrackMixer from './TrackMixer.svelte';
  import type { ButtonId } from './actions';
  import type { ActionPayload } from './action-dispatcher';
  import type { Keymap } from './keymap';
  import type { StemsSnapshot } from './stems';
  let { clickEnabled, clickLevel, clickAvailable, hasMap, stems, stemsDisabled, frozen, keymap, available, run, onclicklevel }:
    { clickEnabled: boolean; clickLevel: number; clickAvailable: boolean; hasMap: boolean; stems: StemsSnapshot | null; stemsDisabled: boolean;
      frozen: boolean; keymap: Keymap; available: (id: ButtonId) => boolean; run: (id: ButtonId, payload?: ActionPayload) => void; onclicklevel: (level: number) => void } = $props();
  const clickTitle = $derived(hasMap ? keymap.title('practice_metronome') : 'The click needs a measure map');
  const clickOn = $derived(clickEnabled && hasMap);
</script>

<div class="mix" role="group" aria-label="Mix" title={frozen ? 'The mixer is unavailable while the song is split into stems' : undefined}>
  <Knob label="Click" value={clickLevel / 100} disabled={frozen || !hasMap} dimmed={!clickOn} title={hasMap ? undefined : clickTitle} onchange={value => onclicklevel(Math.round(value * 100))}>
    <button class="click-toggle" aria-pressed={clickOn} aria-disabled={!clickAvailable} title={clickTitle} onclick={() => { if (clickAvailable) run('practice_metronome'); }}>{keymap.buttonLabel('practice_metronome', clickOn ? 'On' : 'Off')}</button>
  </Knob>
  <span class="divider" aria-hidden="true"></span>
  {#if stems?.set_id}
    <TrackMixer tracks={stems.tracks} disabled={stemsDisabled} {keymap} ongain={(role, gain) => run('stem_gain', { role, gain })} onmute={role => run(`mute_${role}`)} onsolo={role => run(`solo_${role}`)}/>
  {:else}
    <div class="split">
      <span>Want to hear one instrument on its own?</span>
      <button class="link" disabled={!available('show_stems')} title={keymap.title('show_stems')} onclick={() => run('show_stems')}>Split into stems ›</button>
    </div>
  {/if}
</div>

<style>
  .mix{display:flex;align-items:center;gap:2px}
  .divider{align-self:stretch;width:1px;margin:4px 8px;background:var(--border)}
  .click-toggle{width:51px;height:22px;padding:0;border:1px solid var(--border);border-radius:4px;background:var(--surface);color:var(--text-secondary);font-size:10px;font-weight:700}
  .click-toggle[aria-disabled=true]{opacity:.45}
  .click-toggle[aria-pressed=true]{border-color:var(--blue);background:var(--blue-wash);color:var(--blue)}
  .split{width:150px;display:flex;flex-direction:column;align-items:flex-start;gap:4px;padding-left:10px;font-size:12px;line-height:1.35;color:var(--text-secondary)}
  .link{border:0;padding:0;background:transparent;color:var(--link);font-weight:600;font-size:12px}
  .link:hover:not(:disabled){text-decoration:underline}
</style>
