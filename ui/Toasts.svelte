<script lang="ts">
  // Global notices as toasts below the header. Errors persist until resolved
  // or dismissed, informational notices time out, progress stays while it runs.
  import { onDestroy } from 'svelte';
  import { SvelteSet } from 'svelte/reactivity';
  import Icon from './Icon.svelte';
  import ProgressRing from './ProgressRing.svelte';
  import { INFO_TIMEOUT_MS, type Notice } from './notices';
  import type { ButtonId } from './actions';
  import type { Keymap } from './keymap';
  let { notices, keymap, run }: { notices: Notice[]; keymap: Keymap; run: (id: ButtonId) => void } = $props();

  const hidden = new SvelteSet<string>();
  const timers = new Map<string, ReturnType<typeof setTimeout>>();
  const shown = $derived(notices.filter(notice => !hidden.has(notice.key)));
  $effect(() => {
    const keys = new Set(notices.map(notice => notice.key));
    for (const [key, timer] of timers) if (!keys.has(key)) { clearTimeout(timer); timers.delete(key); }
    for (const notice of notices) {
      if (notice.tone === 'info' && !timers.has(notice.key)) timers.set(notice.key, setTimeout(() => hidden.add(notice.key), INFO_TIMEOUT_MS));
    }
  });
  // A message that went away may come back later: show it again, with a fresh timeout.
  $effect(() => { for (const key of hidden) if (!notices.some(notice => notice.key === key)) hidden.delete(key); });
  onDestroy(() => timers.forEach(clearTimeout));

  function dismiss(notice: Notice) {
    if (notice.dismiss === 'hide') hidden.add(notice.key);
    else if (notice.dismiss) run(notice.dismiss);
  }
</script>

<div class="toasts">
  {#each shown as notice (notice.key)}
    <div class="toast {notice.tone}" role={notice.tone === 'error' ? 'alert' : 'status'}>
      {#if notice.tone === 'progress'}<ProgressRing value={notice.progress ?? null}/>{/if}
      <span class="text copyable">{#if notice.title}<b>{notice.title}</b>{' '}{/if}{notice.text}</span>
      {#if notice.action}{@const button = notice.action.button}
        <button class="action" disabled={!notice.action.enabled} title={keymap.title(button)} onclick={() => run(button)}>{notice.action.label}</button>
      {/if}
      {#if notice.dismiss}<button class="dismiss" aria-label="Dismiss" title={keymap.title('dismiss_error')} onclick={() => dismiss(notice)}><Icon name="close" size={14}/></button>{/if}
    </div>
  {/each}
</div>

<style>
  .toasts{position:absolute;z-index:30;top:62px;left:50%;transform:translateX(-50%);width:460px;max-width:calc(100% - 32px);display:flex;flex-direction:column;gap:8px;pointer-events:none}
  .toast{pointer-events:auto;display:flex;align-items:center;gap:10px;padding:10px 10px 10px 14px;border:1px solid var(--border-strong);border-radius:8px;background:var(--surface);color:var(--text);box-shadow:0 8px 24px var(--dialog-shadow);font-size:12px;line-height:1.45}
  .toast.error{border-color:var(--error-border);background:var(--error-bg);color:var(--error-text)}
  .text{flex:1;min-width:0;white-space:pre-line;overflow-wrap:anywhere}
  .progress .text b{display:block}
  .action{flex:none;height:28px;padding:0 10px;border:1px solid var(--border);border-radius:5px;background:var(--surface);color:inherit;font-size:12px;font-weight:600}
  .error .action{border-color:var(--error-border)}
  .action:hover:not(:disabled){background:var(--hover)}
  .dismiss{flex:none;display:grid;place-items:center;width:24px;height:24px;padding:0;border:0;border-radius:4px;background:transparent;color:inherit}
  .dismiss:hover{background:var(--hover)}
</style>
