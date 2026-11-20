<script lang="ts">
  // Settings → Audio (G1–G3): the output in use and Retry when it's lost. The
  // device chooser is drawn for the future (#18) and does nothing yet.
  import Icon from './Icon.svelte';
  import SettingsButton from './SettingsButton.svelte';
  import type { ButtonId } from './actions';
  import type { Keymap } from './keymap';
  let { device, audioError, native, keymap, available, run }:
    { device: string | null; audioError: string | null; native: boolean; keymap: Keymap; available: (id: ButtonId) => boolean; run: (id: ButtonId) => void } = $props();
</script>

<section class="settings-section" aria-labelledby="settings-audio">
  <h3 id="settings-audio">Audio</h3>
  {#if audioError}
    <div class="settings-alert" role="alert"><Icon name="warning" size={15}/><span><b>Playback stopped.</b> {audioError}. Reconnect the device or switch the system output, then retry.</span></div>
    <div class="settings-actions"><SettingsButton id="retry_audio" label="Retry audio" variant="primary" {keymap} {available} {run}/></div>
  {:else if device}
    <div class="settings-row"><span class="label"><strong class="copyable">{device}</strong>
      <span class="settings-hint tone-success in-use"><Icon name="check" size={13}/>In use · system default output</span></span></div>
  {:else}
    <div class="settings-row"><span class="label"><strong>{native ? 'No audio output' : 'Browser preview'}</strong>
      <span class="settings-hint">{native ? 'Open a song to start the system default output.' : 'Audio plays in the desktop app.'}</span></span></div>
  {/if}
  <fieldset class="future" disabled aria-describedby="settings-audio-future">
    <legend>Future · #18</legend>
    <strong>Output device</strong>
    <label><input type="radio" name="output-device" checked/> System default {#if device}<span class="settings-hint">({device})</span>{/if}</label>
    <label><input type="radio" name="output-device"/> Other connected outputs</label>
    <span id="settings-audio-future" class="settings-hint">Choosing an output is planned. Until then, playback follows the system default; change it in your system’s sound settings.</span>
  </fieldset>
</section>

<style>
  .in-use{display:inline-flex;align-items:center;gap:5px}
  .future{margin:8px 0 0;display:flex;flex-direction:column;gap:4px;padding:10px 12px 12px;border:1px dashed var(--border-strong);border-radius:8px;color:var(--text-secondary)}
  legend{padding:0 6px;font-size:11px;font-weight:700;letter-spacing:.06em;text-transform:uppercase;color:var(--link)}
  .future strong{font-size:13px;color:var(--text)}
  label{display:flex;flex-direction:row;align-items:center;gap:8px;margin:0;font-size:13px;font-weight:400}
  input{width:auto;margin:0;padding:0}
</style>
