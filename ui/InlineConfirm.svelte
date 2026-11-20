<script lang="ts">
  // A removal confirmation shown in place, below the row it confirms. It is
  // one Esc level in its dialog; Keep is the default, so it takes keyboard
  // focus during keyboard navigation.
  import { onMount, tick } from 'svelte';
  import { transientRegistrar } from './transient';
  import { currentModality } from './input-modality';
  let { label, text, confirmLabel, disabled = false, onconfirm, oncancel }:
    { label: string; text: string; confirmLabel: string; disabled?: boolean; onconfirm: () => void; oncancel: () => void } = $props();
  const register = transientRegistrar();
  let keep = $state<HTMLButtonElement>();
  onMount(() => {
    const release = register(oncancel);
    if (currentModality() === 'keyboard') void tick().then(() => keep?.focus());
    return release;
  });
</script>

<div class="confirm" role="group" aria-label={label}>
  <p>{text}</p>
  <div class="settings-actions">
    <button bind:this={keep} class="compact-button" onclick={oncancel}>Keep</button>
    <button class="compact-button danger-fill" {disabled} onclick={onconfirm}>{confirmLabel}</button>
  </div>
</div>

<style>
  .confirm{display:flex;flex-direction:column;gap:8px;margin:0 0 6px;padding:10px 12px;border:1px solid var(--border-strong);border-radius:8px;background:var(--sidebar)}
  p{font-size:12px;line-height:1.5;color:var(--text)}
  .danger-fill{border-color:var(--error-text);background:var(--error-text);color:var(--on-accent)}
  .danger-fill:hover:not(:disabled){filter:brightness(1.08);background:var(--error-text)}
</style>
