<script lang="ts">
  import type {RecordingOption} from './types';
  import type {Keymap} from './keymap';
  import type {ButtonId} from './actions';
  import type {ActionPayload} from './action-dispatcher';
  let {recordings,keymap,run}:{recordings:RecordingOption[];keymap:Keymap;run:(id:ButtonId,payload?:ActionPayload)=>void}=$props();
  let dialog:HTMLDialogElement;
  let assetId=$state('');
  export function showModal(){assetId='';dialog.showModal();}
  export function close(){dialog.close();}
  export function isOpen(){return dialog?.open??false;}
  const previous=$derived(recordings.filter(recording=>!recording.current));
</script>
<dialog bind:this={dialog} aria-label="Replace reference recording">
  <h2>Replace reference recording</h2>
  <p>The song keeps its notes and settings. Its previous recording and timeline are retained.</p>
  <p>Reuse the current timeline only if you know the recording has unchanged timing, such as a volume-adjusted copy. Matching duration alone does not establish alignment.</p>
  <button class="button" title={keymap.title('replacement_same_timing')} onclick={()=>run('replacement_same_timing')}>Choose timing-compatible recording</button>
  <button class="button" title={keymap.title('replacement_new_timeline')} onclick={()=>run('replacement_new_timeline')}>Choose recording with new timeline</button>
  <button class="text-button" title={keymap.title('replacement_cancel')} onclick={()=>run('replacement_cancel')}>Cancel</button>
  {#if previous.length}
    <h3>Retained recordings</h3><p>Switching restores the selected recording’s timeline, sections and map.</p>
    <label>Recording<select aria-label="Retained recording" bind:value={assetId}><option value="">Choose recording</option>{#each previous as recording}<option value={recording.asset_id}>{recording.path??'Built-in demo'}</option>{/each}</select></label>
    <button class="button" disabled={!assetId} title={keymap.title('replacement_select')} onclick={()=>run('replacement_select',{assetId})}>Use retained recording</button>
  {/if}
</dialog>
