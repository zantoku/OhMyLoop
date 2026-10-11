<script lang="ts">
  import Icon from './Icon.svelte';
  import type {NotesState} from './notes-session';
  import type {Keymap} from './keymap';
  import type {ButtonId} from './actions';
  let {active,state,keymap,available,onchange,run}:{active:boolean;state:NotesState;keymap:Keymap;available:(id:ButtonId)=>boolean;onchange:(text:string)=>void;run:(id:ButtonId)=>void}=$props();
</script>
{#if active}
<main class="notes-screen" aria-label="Song notes">
  <div class="notes-heading"><div class="screen-title"><button class="icon-button screen-close" aria-label={keymap.buttonLabel('close_notes','Close notes')} title={keymap.title('close_notes')} disabled={!available('close_notes')} onclick={()=>run('close_notes')}><Icon name="close"/></button><h2>Song notes</h2></div><span role="status">{state.pending?'Saving…':state.dirty?'Unsaved changes':'Saved'}</span></div>
  {#if state.error}<div class="notes-error" role="alert"><span>{state.error}</span><button class="text-button" disabled={!available('retry_notes')} title={keymap.title('retry_notes')} onclick={()=>run('retry_notes')}>{keymap.buttonLabel('retry_notes','Retry save')}</button></div>{/if}
  <textarea aria-label="Notes for this song" placeholder="Add notes for this song…" value={state.text} oninput={event=>onchange(event.currentTarget.value)} spellcheck="true"></textarea>
</main>
{/if}
<style>
  .notes-screen{display:flex;flex-direction:column;flex:1;min-height:0;background:#fff;border:1px solid #e1e6f0;border-radius:9px;padding:22px 25px;gap:18px}.notes-heading{display:flex;align-items:center;justify-content:space-between;gap:15px}.notes-heading h2{font-size:23px;font-weight:500;letter-spacing:-.5px}.notes-heading span{color:#66877a;font-size:11px}textarea{display:block;flex:1;min-height:150px;width:100%;resize:none;border:1px solid #d3ddeb;border-radius:5px;padding:16px 18px;font:inherit;line-height:1.7;font-size:14px;color:#24314c;background:#fdfdfe}textarea:focus-visible{outline:3px solid #8297d2;outline-offset:3px}.notes-error{display:flex;justify-content:space-between;align-items:center;gap:15px;padding:10px 12px;border-radius:5px;color:#963e57;background:#fbebef;font-size:12px}.notes-error>span{overflow-wrap:anywhere}.notes-error button{white-space:nowrap}
  @media(max-width:1000px){.notes-screen{padding:18px 17px}}
</style>
