<script lang="ts">
  // The app shell's header: a breadcrumb from the Library to the open song,
  // the song's views as a segmented control, and screen-wide tools.
  import type { Snippet } from 'svelte';
  import type { ButtonId } from './actions';
  import type { Keymap } from './keymap';
  import ViewSwitcher, { type ViewSegment } from './ViewSwitcher.svelte';
  let { title, inLibrary, views, keymap, available, run, children }:
    { title: string | null; inLibrary: boolean; views: ViewSegment[]; keymap: Keymap;
      available: (id: ButtonId) => boolean; run: (id: ButtonId) => void; children?: Snippet } = $props();
</script>

<header class="app-header">
  <nav class="breadcrumb" aria-label="Breadcrumb">
    {#if inLibrary || !title}<h1 aria-current="page">Library</h1>
    {:else}
      <button class="crumb" disabled={!available('show_library')} title={keymap.title('show_library')} onclick={() => run('show_library')}>Library</button>
      <span aria-hidden="true">›</span><h1 aria-current="page">{title}</h1>
    {/if}
  </nav>
  {#if views.length}<ViewSwitcher label="Song views" segments={views} {keymap} {available} {run}/>{:else}<span></span>{/if}
  <div class="header-tools">{@render children?.()}</div>
</header>

<style>
  .app-header{height:50px;flex:none;display:grid;grid-template-columns:minmax(0,1fr) auto minmax(0,1fr);align-items:center;gap:16px;padding:0 18px;background:var(--surface);border-bottom:1px solid var(--border)}
  .breadcrumb{display:flex;align-items:center;gap:8px;min-width:0;font-size:13px;color:var(--text-secondary)}
  h1{font-size:13px;font-weight:700;letter-spacing:0;color:var(--text);white-space:nowrap;overflow:hidden;text-overflow:ellipsis}
  .crumb{border:0;padding:3px 4px;margin-left:-4px;border-radius:4px;background:transparent;color:var(--text-secondary);font-weight:600}
  .crumb:hover:not(:disabled){color:var(--text);background:var(--hover)}
  .header-tools{display:flex;justify-content:flex-end;align-items:center;gap:14px;min-width:0;font-size:11px;color:var(--text-secondary)}
</style>
