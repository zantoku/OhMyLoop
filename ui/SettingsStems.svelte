<script lang="ts">
  // Settings → Stem separation (C1–C4): status, size and install with
  // progress and Cancel; repair; and removal with the reassurance that
  // stems stay. Every command is a `separation_*` action.
  import Icon from './Icon.svelte';
  import SettingsButton from './SettingsButton.svelte';
  import SettingsProgress from './SettingsProgress.svelte';
  import InlineConfirm from './InlineConfirm.svelte';
  import { megabytes, stemTools } from './settings';
  import type { SeparationState } from './separation';
  import type { ButtonId } from './actions';
  import type { Keymap } from './keymap';
  let { separation, locked, keymap, available, run }:
    { separation: SeparationState; locked: boolean; keymap: Keymap; available: (id: ButtonId) => boolean; run: (id: ButtonId) => void } = $props();
  const status = $derived(separation.status);
  const tools = $derived(stemTools(status));
  const size = $derived(status ? megabytes(status.package_bytes) : '');
  const failure = $derived(separation.error || (status?.install.phase === 'Failed' ? status.install.error : null));
  let confirming = $state<'runtime' | 'model' | null>(null);
  const shared = $derived({ keymap, available, run, locked });
  function remove(id: 'separation_remove_runtime' | 'separation_remove_model') { confirming = null; run(id); }
</script>

<section class="settings-section" aria-labelledby="settings-stems">
  <h3 id="settings-stems">Stem separation</h3>
  <span class="settings-intro">Splits songs into instrument tracks, offline. Shared by every song and library; removing it keeps all stems.</span>
  {#if tools.kind === 'installing'}
    <div class="settings-row"><span class="label"><strong>Separation tools and model</strong><span class="settings-hint" role="status">{tools.phase}</span></span></div>
    <SettingsProgress label="Stem separation setup progress" progress={tools.progress} hint="Playback keeps working while it downloads. Other settings wait until it’s done.">
      <SettingsButton id="separation_cancel" label={tools.phase === 'Stopping' ? 'Stopping…' : 'Cancel'} {keymap} {available} {run}/>
    </SettingsProgress>
  {:else if tools.kind === 'checking'}
    <div class="settings-row"><span class="settings-hint" role="status">Checking the separation tools…</span></div>
  {:else if tools.kind === 'unsupported'}
    <div class="settings-row"><span class="label"><strong>Separation tools</strong><span class="settings-hint">{status?.runtime.message}</span></span></div>
  {:else if tools.kind === 'none' || tools.kind === 'failed'}
    <div class="settings-row"><span class="label"><strong>Separation tools and model</strong><span class="settings-hint">Not installed · {size} model download, plus the separation tools</span></span>
      <SettingsButton id="separation_install" label={tools.kind === 'failed' ? 'Retry' : 'Install'} variant="primary" {...shared}/></div>
  {:else}
    <div class="settings-row"><span class="label"><strong>Separation tools</strong>
      {#if status?.runtime.ready}<span class="settings-hint tone-success">Installed and verified</span>
      {:else}<span class="settings-hint tone-warning damaged"><Icon name="warning" size={13}/>{status?.runtime.path ? `Needs repair: ${status.runtime.message}` : 'Not installed'}</span>{/if}</span>
      <SettingsButton id="separation_reinstall_runtime" label={status?.runtime.path ? 'Reinstall' : 'Install'} variant={status?.runtime.ready ? 'ghost' : 'primary'} {...shared}/>
      {#if status?.runtime.path}<SettingsButton id="separation_remove_runtime" label="Remove…" variant="danger" {...shared} onclick={() => { confirming = 'runtime'; }}/>{/if}
    </div>
    {#if confirming === 'runtime'}<InlineConfirm label="Remove separation tools" confirmLabel="Remove tools" disabled={locked || !available('separation_remove_runtime')}
      text="The separation tools and their download cache are removed. The model, recordings, completed stems, notes and maps stay. You can install the tools again later."
      onconfirm={() => remove('separation_remove_runtime')} oncancel={() => { confirming = null; }}/>{/if}
    <div class="settings-row"><span class="label"><strong>Roformer-SW model</strong>
      {#if status?.model_ready}<span class="settings-hint tone-success">Installed · {size}</span>
      {:else}<span class="settings-hint tone-warning damaged"><Icon name="warning" size={13}/>Missing · {size} download</span>{/if}</span>
      <SettingsButton id="separation_reinstall_model" label={status?.model_ready ? 'Reinstall' : 'Install'} variant={status?.model_ready ? 'ghost' : 'primary'} {...shared}/>
      {#if status?.model_ready}<SettingsButton id="separation_remove_model" label="Remove…" variant="danger" {...shared} onclick={() => { confirming = 'model'; }}/>{/if}
    </div>
    {#if confirming === 'model'}<InlineConfirm label="Remove the separation model" confirmLabel="Remove model" disabled={locked || !available('separation_remove_model')}
      text="The downloaded model is removed. Completed stems, recordings, notes and maps stay. Separating another song needs the model again."
      onconfirm={() => remove('separation_remove_model')} oncancel={() => { confirming = null; }}/>{/if}
    <div><SettingsButton id="separation_refresh_runtime" label="Check again" variant="ghost" {...shared}/></div>
  {/if}
  {#if failure}<div class="settings-alert" role="alert"><Icon name="warning" size={15}/><span><b>Setup failed:</b> {failure}</span></div>{/if}
  {#if status}
    <details><summary>Storage location</summary>
      <p>Model: <code class="settings-path">{status.model_path}</code></p>
      {#if status.runtime.path}<p>Tools: <code class="settings-path">{status.runtime.path}</code></p>{/if}
    </details>
  {/if}
</section>

<style>
  .damaged{display:inline-flex;align-items:center;gap:5px}
</style>
