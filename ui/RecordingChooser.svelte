<script lang="ts">
  import Icon from './Icon.svelte';
  import type {Keymap} from './keymap';
  import type {ButtonId} from './actions';
  let {keymap,run}:{keymap:Keymap;run:(id:ButtonId)=>void}=$props();
  let dialog:HTMLDialogElement;
  export function showModal(){dialog.showModal();}
  export function close(){dialog.close();}
  export function isOpen(){return dialog?.open??false;}
</script>
<dialog bind:this={dialog} aria-label="Replace reference recording" oncancel={event=>{event.preventDefault();run('replacement_cancel');}}>
  <div class="replacement-heading">
    <button class="icon-button" aria-label={keymap.buttonLabel('replacement_cancel','Close recording replacement')} title={keymap.title('replacement_cancel')} onclick={()=>run('replacement_cancel')}><Icon name="close"/></button>
    <h2>Replace reference recording</h2>
  </div>
  <div class="replacement-copy">
    <p>Replacing the reference removes any attached stems and their playback cache, even when timing is unchanged.</p>
    <p>The song keeps its notes and speed/pitch settings.</p>
    <p>Keep the current map, sections and loop only if the recording has unchanged timing, such as a volume-adjusted copy. Matching duration alone does not establish alignment.</p>
  </div>
  <div class="replacement-actions">
    <button class="button" title={keymap.title('replacement_same_timing')} onclick={()=>run('replacement_same_timing')}>Choose timing-compatible recording</button>
    <div class="changed-timing">
      <p>Changed timing clears the measure map, sections, loop and saved position. This cannot be undone.</p>
      <button class="button" title={keymap.title('replacement_new_timeline')} onclick={()=>run('replacement_new_timeline')}>Choose recording with changed timing</button>
    </div>
  </div>
</dialog>
<style>
  .replacement-heading{display:flex;align-items:flex-start;gap:12px}
  .replacement-heading .icon-button{flex-shrink:0;margin-inline-start:-6px;margin-block-start:-3px}
  .replacement-heading h2{line-height:1.4}
  .replacement-copy{display:grid;gap:12px;margin-block:20px}
  .replacement-actions{display:grid;gap:20px}
  .changed-timing{display:grid;gap:10px}
  p{margin:0}
  .replacement-actions .button{width:100%;white-space:normal;text-align:start;justify-content:flex-start;line-height:1.5}
</style>
