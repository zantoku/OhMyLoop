<script lang="ts" module>
  import type { ButtonId } from './actions';
  export type ViewSegment = { button: ButtonId; label: string; current: boolean };
</script>

<script lang="ts">
  // A segmented control that switches between views. Each segment runs an
  // action, so its tooltip and availability come from the keymap model.
  import type { Keymap } from './keymap';
  let { label, segments, keymap, available, run }:
    { label: string; segments: ViewSegment[]; keymap: Keymap; available: (id: ButtonId) => boolean; run: (id: ButtonId) => void } = $props();
</script>

<div class="segmented" role="group" aria-label={label}>
  {#each segments as segment (segment.button)}
    <button aria-current={segment.current ? 'page' : undefined} disabled={!segment.current && !available(segment.button)}
      title={keymap.title(segment.button)} onclick={() => { if (!segment.current) run(segment.button); }}>{keymap.buttonLabel(segment.button, segment.label)}</button>
  {/each}
</div>

<style>
  .segmented{display:flex;gap:2px;padding:3px;border:1px solid var(--border);border-radius:8px;background:var(--page)}
  button{padding:6px 14px;border:0;border-radius:6px;background:transparent;color:var(--text-secondary);font-size:12px;font-weight:600;white-space:nowrap}
  button:hover:not(:disabled,[aria-current]){color:var(--text);background:var(--hover)}
  button[aria-current]{background:var(--surface);color:var(--text);font-weight:700;box-shadow:0 1px 2px var(--dialog-shadow)}
</style>
