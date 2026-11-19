<script lang="ts">
  // The window-wide drop target shown while a file is dragged over the app.
  import Icon from './Icon.svelte';
  import type { DropPreview } from './file-drop';
  let { preview }: { preview: DropPreview } = $props();
</script>

<div class="drop" class:refused={'refused' in preview} aria-hidden="true">
  <div class="card">
    <Icon name={'refused' in preview ? 'warning' : 'music'} size={30}/>
    {#if 'refused' in preview}<strong>{preview.refused}</strong><span>Importing several files or a folder isn’t supported yet.</span>
    {:else}<strong>Drop to import “{preview.name}”</strong><span>It’s copied into your library and opens in Practice.</span>{/if}
  </div>
</div>

<style>
  .drop{position:absolute;z-index:40;inset:58px 8px 8px;display:flex;align-items:center;justify-content:center;border:3px dashed var(--blue);border-radius:16px;background:color-mix(in srgb,var(--page) 82%,transparent);pointer-events:none}
  .drop.refused{border-color:var(--error-text)}
  .card{display:flex;flex-direction:column;align-items:center;gap:8px;padding:24px 32px;border-radius:12px;background:var(--surface);box-shadow:0 10px 30px var(--dialog-shadow);color:var(--blue)}
  .refused .card{color:var(--error-text)}
  strong{font-size:16px;color:var(--text)}
  span{font-size:12px;color:var(--text-secondary)}
</style>
