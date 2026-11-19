<script lang="ts">
  // A download's progress in Settings: a bar (determinate when the size is
  // known), its percentage, Cancel, and what still works meanwhile.
  import type { Snippet } from 'svelte';
  let { label, progress, hint, children }: { label: string; progress: number | null; hint: string; children?: Snippet } = $props();
</script>

<div class="progress">
  <div class="bar">
    {#if progress === null}<progress aria-label={label}></progress>{:else}<progress aria-label={label} max="1" value={progress}></progress>{/if}
    {#if progress !== null}<span class="settings-hint">{Math.round(progress * 100)} %</span>{/if}
    {@render children?.()}
  </div>
  <span class="settings-hint">{hint}</span>
</div>

<style>
  .progress{display:flex;flex-direction:column;gap:6px;padding:10px 0;border-top:1px solid var(--border)}
  .bar{display:flex;align-items:center;gap:10px}
  progress{flex:1;height:6px;border:0;border-radius:3px;overflow:hidden;background:var(--border);accent-color:var(--blue)}
  progress::-webkit-progress-bar{background:var(--border);border-radius:3px}
  progress::-webkit-progress-value{background:var(--blue);border-radius:3px}
</style>
