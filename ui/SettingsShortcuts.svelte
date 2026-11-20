<script lang="ts">
  // Settings → Shortcuts (F1–F3): the Practice shortcuts button, the full
  // list, and the shortcut file with its location, Open, Reload and status.
  // The app reads keymap.json but never writes it (docs/KEYMAP.md).
  import Icon from './Icon.svelte';
  import SettingsButton from './SettingsButton.svelte';
  import type { KeymapSource } from './keymap';
  import type { ButtonId } from './actions';
  import type { Keymap } from './keymap';
  let { showButton, source, path, warning, loading, revealLabel, locked, saveFailed, onshowbutton, keymap, available, run }:
    { showButton: boolean; source: KeymapSource; path: string | null; warning: string | null; loading: boolean; revealLabel: string; locked: boolean;
      saveFailed: boolean; onshowbutton: (shown: boolean) => void; keymap: Keymap; available: (id: ButtonId) => boolean; run: (id: ButtonId) => void } = $props();
  const shared = $derived({ keymap, available, run, locked });
</script>

<section class="settings-section" aria-labelledby="settings-shortcuts">
  <h3 id="settings-shortcuts">Shortcuts</h3>
  <div class="settings-row">
    <label class="check"><input type="checkbox" checked={showButton} onchange={event => onshowbutton(event.currentTarget.checked)}/>Show the shortcuts button on Practice</label>
    <SettingsButton id="shortcuts" label="View all shortcuts" keymap={keymap} {available} {run}/>
  </div>
  {#if saveFailed}<span class="settings-hint tone-error" role="status">This choice couldn’t be saved. It applies until you quit.</span>{/if}
  <div class="settings-row file">
    <strong>Shortcut file</strong>
    {#if path}
      <code class="settings-path">{path}</code>
      {#if source === 'builtin'}<span class="settings-intro">No file yet: the built-in shortcuts are used. Copy the example keymap.json from the OhMyLoop documentation to this path to change them.</span>
      {:else if source === 'custom'}<span class="settings-hint tone-success loaded"><Icon name="check" size={13}/>Custom shortcuts loaded</span>
        {#if warning}<div class="settings-alert warning" role="status"><Icon name="warning" size={15}/><span>{warning}</span></div>{/if}
      {:else}<div class="settings-alert" role="alert"><Icon name="warning" size={15}/><span><b>keymap.json was rejected:</b> {warning} The built-in shortcuts are used until it’s fixed.</span></div>{/if}
      <div class="settings-actions">
        <SettingsButton id="open_keymap" label="Open file" {...shared}/>
        <SettingsButton id="reveal_keymap" label={revealLabel} {...shared}/>
        <SettingsButton id="reload_keymap" label={loading ? 'Reloading…' : 'Reload'} {...shared}/>
      </div>
    {:else}<span class="settings-intro">The shortcut file is available in the desktop app.</span>{/if}
  </div>
</section>

<style>
  .check{flex:1;display:flex;flex-direction:row;align-items:center;gap:8px;margin:0;font-size:13px;font-weight:400}
  .check input{width:auto;margin:0;padding:0}
  .file{flex-direction:column;align-items:stretch;gap:6px}
  .loaded{display:inline-flex;align-items:center;gap:5px;font-size:12px}
</style>
