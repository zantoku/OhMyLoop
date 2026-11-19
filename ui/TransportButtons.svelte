<script lang="ts">
  // Step back, Play/Pause and step forward. Steps move by a measure, or by
  // 5 seconds in a song without a measure map.
  import Icon from './Icon.svelte';
  import type { ButtonId } from './actions';
  import type { Keymap } from './keymap';
  let { playing, mapped, frozen, keymap, available, run }:
    { playing: boolean; mapped: boolean; frozen: boolean; keymap: Keymap; available: (id: ButtonId) => boolean; run: (id: ButtonId) => void } = $props();
  const frozenTitle = 'Playback is paused while the song is split into stems';
</script>

<div class="transport" role="group" aria-label="Transport">
  <button class="step" aria-label={keymap.buttonLabel('step_back', mapped ? 'Previous measure' : 'Back 5 seconds')} disabled={!available('step_back')} title={keymap.title('step_back')} onclick={() => run('step_back')}><Icon name="back" size={22}/></button>
  <button class="play" aria-label={keymap.buttonLabel('play', playing ? 'Pause' : 'Play')} disabled={!available('play')} title={frozen ? frozenTitle : keymap.title('play')} onclick={() => run('play')}><Icon name={playing ? 'pause' : 'play'} size={28}/></button>
  <button class="step" aria-label={keymap.buttonLabel('step_forward', mapped ? 'Next measure' : 'Forward 5 seconds')} disabled={!available('step_forward')} title={keymap.title('step_forward')} onclick={() => run('step_forward')}><Icon name="next" size={22}/></button>
</div>

<style>
  .transport{display:flex;align-items:center;gap:6px}
  button{display:grid;place-items:center;height:52px;padding:0;border-radius:8px}
  .step{width:44px;border:1px solid var(--border);background:var(--surface);color:var(--text-secondary)}
  .step:hover:not(:disabled){background:var(--hover);color:var(--text)}
  .play{width:88px;border:0;background:var(--blue);color:var(--on-accent)}
  .play:hover:not(:disabled){background:var(--blue-hover)}
</style>
