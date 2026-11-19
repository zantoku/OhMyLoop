<script lang="ts" generics="T extends string">
  // A segmented choice for a preference (not an action): the pressed segment
  // is the current value, and choosing applies at once.
  let { label, value, options, onchange }:
    { label: string; value: T; options: readonly { value: T; label: string }[]; onchange: (value: T) => void } = $props();
</script>

<span class="choices" role="group" aria-label={label}>
  {#each options as option (option.value)}
    <button aria-pressed={option.value === value} onclick={() => { if (option.value !== value) onchange(option.value); }}>{option.label}</button>
  {/each}
</span>

<style>
  .choices{display:inline-flex;flex:none;gap:2px;padding:2px;border:1px solid var(--border);border-radius:7px;background:var(--page)}
  button{height:24px;padding:0 9px;border:0;border-radius:5px;background:transparent;color:var(--text-secondary);font-size:12px;font-weight:600}
  button:hover:not([aria-pressed=true]){color:var(--text);background:var(--hover)}
  button[aria-pressed=true]{background:var(--surface);color:var(--text);box-shadow:0 1px 2px var(--dialog-shadow)}
</style>
