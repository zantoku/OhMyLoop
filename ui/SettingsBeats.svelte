<script lang="ts">
  // Settings → Beat analysis (D1, D2): install the model with its size and
  // progress, cancel, retry a failed download, and remove it knowing maps and
  // analysis results stay.
  import Icon from './Icon.svelte';
  import SettingsButton from './SettingsButton.svelte';
  import SettingsProgress from './SettingsProgress.svelte';
  import InlineConfirm from './InlineConfirm.svelte';
  import { BEAT_MODEL_BYTES, beatModel, megabytes } from './settings';
  import type { BeatModelState } from './beat-model-session';
  import type { ButtonId } from './actions';
  import type { Keymap } from './keymap';
  let { model: download, locked, keymap, available, run }:
    { model: BeatModelState; locked: boolean; keymap: Keymap; available: (id: ButtonId) => boolean; run: (id: ButtonId) => void } = $props();
  const model = $derived(beatModel(download.status));
  const size = megabytes(BEAT_MODEL_BYTES);
  const failure = $derived(download.error || (model.kind === 'failed' ? download.status?.error : null));
  let confirming = $state(false);
  const shared = $derived({ keymap, available, run, locked });
</script>

<section class="settings-section" aria-labelledby="settings-beats">
  <h3 id="settings-beats">Beat analysis</h3>
  <span class="settings-intro">Finds beats and downbeats to draft a measure map. Removing it keeps your maps and analysis results. Manual mapping needs no download.</span>
  {#if model.kind === 'downloading'}
    <div class="settings-row"><span class="label"><strong>Beat analysis model</strong><span class="settings-hint" role="status">{model.phase}</span></span></div>
    <SettingsProgress label="Beat analysis model download progress" progress={model.progress} hint="Other settings wait until it’s done.">
      <SettingsButton id="map_cancel_model" label="Cancel" {keymap} {available} {run}/>
    </SettingsProgress>
  {:else if model.kind === 'checking'}
    <div class="settings-row"><span class="settings-hint" role="status">Checking the beat analysis model…</span></div>
  {:else if model.kind === 'ready' || model.kind === 'busy'}
    <div class="settings-row"><span class="label"><strong>Beat analysis model</strong>
      <span class="settings-hint tone-success">{model.kind === 'busy' ? 'Installed · analysing a song' : `Installed · about ${size}`}</span></span>
      <SettingsButton id="map_install_model" label="Verify" variant="ghost" {...shared}/>
      <SettingsButton id="map_remove_model" label="Remove…" variant="danger" {...shared} onclick={() => { confirming = true; }}/>
    </div>
    {#if confirming}<InlineConfirm label="Remove the beat analysis model" confirmLabel="Remove model" disabled={locked || !available('map_remove_model')}
      text="The model files are removed. Your measure maps and analysis results stay; analysing another song needs the model again."
      onconfirm={() => { confirming = false; run('map_remove_model'); }} oncancel={() => { confirming = false; }}/>{/if}
  {:else}
    <div class="settings-row"><span class="label"><strong>Beat analysis model</strong><span class="settings-hint">Not installed · about {size} download</span></span>
      <SettingsButton id="map_install_model" label={model.kind === 'failed' ? 'Retry' : 'Install'} variant="primary" {...shared}/></div>
  {/if}
  {#if failure}<div class="settings-alert" role="alert"><Icon name="warning" size={15}/><span><b>{model.kind === 'failed' ? 'Download failed:' : 'Something went wrong:'}</b> {failure}</span></div>{/if}
  {#if download.status}
    <details><summary>Storage location</summary><p><code class="settings-path">{download.status.model_path}</code></p>
      <p>Beat This! uses the MIT license. Installed models work offline.</p></details>
  {/if}
</section>
