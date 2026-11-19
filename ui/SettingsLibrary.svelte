<script lang="ts">
  // Settings → Library (B1–B5): the open library, where it lives and how many
  // songs it holds; switching or creating one; moving it; and recovering a
  // missing library through the same actions as the Library screen's notice.
  import Icon from './Icon.svelte';
  import SettingsButton from './SettingsButton.svelte';
  import { libraryName } from './settings';
  import type { LibraryStatus } from './types';
  import type { ButtonId } from './actions';
  import type { Keymap } from './keymap';
  let { status, songCount, revealLabel, working, locked, keymap, available, run }:
    { status: LibraryStatus; songCount: number | null; revealLabel: string; working: string; locked: boolean;
      keymap: Keymap; available: (id: ButtonId) => boolean; run: (id: ButtonId) => void } = $props();
  const shared = $derived({ keymap, available, run, locked });
</script>

<section class="settings-section" aria-labelledby="settings-library">
  <h3 id="settings-library">Library</h3>
  <span class="settings-intro">Songs, their recordings, notes, maps and stems live in one library folder. One library is open at a time.</span>
  {#if status.selected && status.available}
    <div class="settings-row">
      <span class="folder"><Icon name="folder" size={18}/></span>
      <span class="label"><strong>{libraryName(status.selected.root)}</strong>
        <span class="settings-path">{status.selected.root}{songCount === null ? '' : ` · ${songCount} ${songCount === 1 ? 'song' : 'songs'}`}</span></span>
      <SettingsButton id="reveal_library" label={revealLabel} variant="ghost" {...shared}/>
    </div>
  {:else if status.selected}
    <div class="settings-alert warning" role="alert"><Icon name="warning" size={15}/>
      <span><b>Library not found.</b> <span class="copyable">{status.selected.root}</span>. {status.error ?? 'The drive may be unplugged or the folder moved.'}</span></div>
    <div class="settings-actions"><SettingsButton id="managed_retry" label="Retry" variant="primary" {...shared}/><SettingsButton id="managed_locate" label="Locate…" {...shared}/></div>
  {:else}
    <p class="settings-row settings-hint">You don’t have a library yet. It’s created on your first import, or here.</p>
  {/if}
  <div class="settings-row settings-actions">
    <SettingsButton id="managed_open" label="Open another library…" {...shared}/>
    <SettingsButton id="managed_create" label="Create a new library…" {...shared}/>
  </div>
  <span class="settings-hint" role="status">{working || 'Switching saves your edits and closes the current song.'}</span>
  <details><summary>Moving or backing up a library</summary>
    <p>Quit OhMyLoop, then copy or move the whole library folder. Afterwards, open it here with Open another library. Notes, maps and stems travel with it.</p></details>
</section>

<style>
  .folder{display:flex;color:var(--text-secondary)}
</style>
