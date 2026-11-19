<script lang="ts">
  // Count-in before Play, and Loop with its joined "each" segment for a
  // count-in before every repeat. The three toggles are independent.
  import Icon from './Icon.svelte';
  import type { ButtonId } from './actions';
  import type { Keymap } from './keymap';
  let { countIn, loop, repeatCountIn, mapped, keymap, available, run }:
    { countIn: boolean; loop: boolean; repeatCountIn: boolean; mapped: boolean; keymap: Keymap; available: (id: ButtonId) => boolean; run: (id: ButtonId) => void } = $props();
  const needsMap = 'Count-in needs a measure map';
  // Unavailable toggles stay focusable and keep their reason tooltip (aria-disabled), so clicks are guarded.
  const guarded = (id: ButtonId) => () => { if (available(id)) run(id); };
</script>

<div class="modes" role="group" aria-label="Practice modes">
  <button class="mode" aria-pressed={countIn} aria-disabled={!available('count_in')} title={mapped ? keymap.title('count_in') : needsMap} onclick={guarded('count_in')}>
    <Icon name="hourglass" size={18}/>{keymap.buttonLabel('count_in', 'Count-in')}
  </button>
  <div class="loop" class:on={loop} role="group" aria-label="Loop">
    <button class="mode" aria-pressed={loop} disabled={!available('loop')} title={keymap.title('loop')} onclick={() => run('loop')}><Icon name="loop" size={18}/>{keymap.buttonLabel('loop', 'Loop')}</button>
    <button class="mode each" aria-pressed={repeatCountIn} aria-label={keymap.buttonLabel('repeat_count_in', 'Count in before each repeat')} aria-disabled={!available('repeat_count_in')}
      title={!mapped ? needsMap : loop ? keymap.title('repeat_count_in') : `${keymap.title('repeat_count_in')}. Turn looping on first.`} onclick={guarded('repeat_count_in')}><Icon name="hourglass" size={16}/>each</button>
  </div>
</div>

<style>
  .modes{display:flex;gap:6px}
  .mode{height:52px;min-width:88px;display:flex;flex-direction:column;align-items:center;justify-content:center;gap:3px;padding:0 10px;border:1px solid var(--border);border-radius:8px;background:var(--surface);color:var(--text-secondary);font-size:11px;font-weight:600}
  .mode:hover:not(:disabled,[aria-disabled=true]){background:var(--hover);color:var(--text)}
  .mode[aria-disabled=true]{opacity:.45}
  .mode[aria-pressed=true]{border-color:var(--amber-border);background:var(--amber-bg);color:var(--amber-text);font-weight:700}
  .loop{display:flex;height:52px;border:1px solid var(--border);border-radius:8px;overflow:hidden}
  .loop.on{border-color:var(--amber-border)}
  .loop .mode{height:100%;min-width:80px;border:0;border-radius:0}
  .loop .each{min-width:48px;padding:0 6px;font-size:10px;border-left:1px solid var(--border)}
  .loop.on .each{border-left-color:var(--amber-border)}
</style>
