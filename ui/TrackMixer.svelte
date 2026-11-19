<script lang="ts">
  // One rotary gain knob per stem, with mute and solo toggles.
  import Knob from './Knob.svelte';
  import { roleName, type StemRole, type StemTrack } from './stems';
  import type { Keymap } from './keymap';
  let { tracks, disabled = false, keymap, ongain, onmute, onsolo }:
    { tracks: StemTrack[]; disabled?: boolean; keymap: Keymap; ongain: (role: StemRole, gain: number) => void; onmute: (role: StemRole) => void; onsolo: (role: StemRole) => void } = $props();
</script>

{#each tracks as track (track.role)}
  <Knob label={roleName(track.role)} value={track.gain} {disabled} dimmed={track.mute} onchange={gain => ongain(track.role, gain)}>
    <div class="toggles">
      <button {disabled} class="mute" aria-pressed={track.mute} aria-label={`Mute ${roleName(track.role)}`} title={keymap.title(`mute_${track.role}`)} onclick={() => onmute(track.role)}>M</button>
      <button {disabled} class="solo" aria-pressed={track.solo} aria-label={`Solo ${roleName(track.role)}`} title={keymap.title(`solo_${track.role}`)} onclick={() => onsolo(track.role)}>S</button>
    </div>
  </Knob>
{/each}

<style>
  .toggles{display:flex;gap:3px}
  button{width:24px;height:22px;padding:0;border:1px solid var(--border);border-radius:4px;background:var(--surface);color:var(--text-secondary);font-size:10px;font-weight:700}
  button:hover:not(:disabled){background:var(--hover)}
  .mute[aria-pressed=true]{border-color:var(--error-border);background:var(--error-bg);color:var(--error-text)}
  .solo[aria-pressed=true]{border-color:var(--amber-border);background:var(--amber-bg);color:var(--amber-text)}
</style>
