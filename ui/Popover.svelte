<script lang="ts">
  // A non-modal popover anchored below its trigger. It owns the keys pressed
  // inside it (`data-popover`); Esc, a click outside, or the trigger closes it.
  // Focus moves in only during keyboard navigation and returns when it closes.
  import { tick, type Snippet } from 'svelte';
  import { currentModality } from './input-modality';
  import { focusHome } from './home-focus';
  import { openTransient } from './transient';
  import { isShortField } from './key-routing';
  let { open, label, anchor, align = 'end', width = 300, onclose, children }:
    { open: boolean; label: string; anchor?: HTMLElement; align?: 'start' | 'end'; width?: number; onclose: () => void; children: Snippet } = $props();
  let element = $state<HTMLDivElement>();

  $effect(() => {
    if (!open) return;
    const previous = document.activeElement;
    const release = openTransient(onclose);
    const outside = (event: PointerEvent) => {
      const target = event.target as Node;
      if (!element?.contains(target) && !anchor?.contains(target)) onclose();
    };
    document.addEventListener('pointerdown', outside, true);
    if (currentModality() === 'keyboard') void tick().then(() => element?.querySelector<HTMLElement>('input, button:not(:disabled)')?.focus());
    return () => {
      release();
      document.removeEventListener('pointerdown', outside, true);
      const active = document.activeElement;
      if (!active || active === document.body || element?.contains(active)) {
        if (previous instanceof HTMLElement && previous.isConnected && previous !== document.body) previous.focus({ preventScroll: true });
        else focusHome();
      }
    };
  });

  function key(event: KeyboardEvent) {
    if (event.key !== 'Escape' || event.isComposing) return;
    // In a field, field-commit first restores its value and sends focus home;
    // the popover closes once that has happened, so nothing commits.
    if (isShortField(event.target as Element)) { setTimeout(onclose); return; }
    event.preventDefault(); event.stopPropagation();
    onclose();
  }
</script>

{#if open}
  <div class="popover {align}" role="dialog" tabindex="-1" aria-label={label} data-popover style:width={`${width}px`} bind:this={element} onkeydown={key}>{@render children()}</div>
{/if}

<style>
  .popover{position:absolute;z-index:25;top:calc(100% + 8px);display:flex;flex-direction:column;gap:12px;padding:14px;border:1px solid var(--border);border-radius:10px;background:var(--surface);color:var(--text);box-shadow:0 10px 28px var(--dialog-shadow);cursor:default}
  .start{left:0}.end{right:0}
</style>
