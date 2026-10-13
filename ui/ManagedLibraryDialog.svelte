<script lang="ts">
  import Icon from './Icon.svelte';
  import type {ButtonId} from './actions';
  import type {Keymap} from './keymap';
  import type {LibraryEntry} from './library';
  let {kind,keymap,run,busy=false,error='',candidates=[],songTitle=''}:{kind:'setup'|'settings'|'notice'|'duplicates'|'delete'|null;keymap:Keymap;busy?:boolean;error?:string;run:(id:ButtonId,songKey?:string)=>void;candidates?:LibraryEntry[];songTitle?:string}=$props();
  let dialog:HTMLDialogElement;
  $effect(()=>{if(kind&&!dialog.open)dialog.showModal();else if(!kind&&dialog.open)dialog.close();});
  const title=$derived(kind==='setup'?'Set up your library':kind==='settings'?'Library settings':kind==='notice'?'Import your music':kind==='duplicates'?'Recording already imported':'Delete song?');
</script>
<dialog bind:this={dialog} aria-label={title} oncancel={event=>{event.preventDefault();if(!busy)run('managed_close');}}>
  <div class="heading"><button disabled={busy} class="icon-button" aria-label="Close library dialog" title={keymap.title('managed_close')} onclick={()=>run('managed_close')}><Icon name="close"/></button><h2>{title}</h2></div>
  {#if kind&&error}<p class="dialog-error" role="alert">{error}</p>{/if}
  {#if kind==='setup'||kind==='notice'}<p>Recordings are copied into your library. Your original files remain untouched.</p>{/if}
  {#if kind==='setup'||kind==='settings'}
    <p>Create an OhMyLoop Library subfolder in a folder you choose, or open an existing library.</p>
    <div class="choices"><button disabled={busy} class="button" title={keymap.title('managed_create')} onclick={()=>run('managed_create')}>Create library</button><button disabled={busy} class="button" title={keymap.title('managed_open')} onclick={()=>run('managed_open')}>Open existing library</button></div>
  {:else if kind==='notice'}<button disabled={busy} class="button primary" title={keymap.title('managed_continue')} onclick={()=>run('managed_continue')}>Continue</button>
  {:else if kind==='duplicates'}
    <p>This exact recording is already in your library. Open an existing song or create independent notes and a measure map with a separate copy.</p>
    <div class="choices">{#each candidates as entry}<button disabled={busy} class="button" title={keymap.title('managed_duplicate_open')} onclick={()=>run('managed_duplicate_open',entry.song_key)}>Open existing song: {entry.metadata.title}</button>{/each}<button disabled={busy} class="button" title={keymap.title('managed_duplicate_import')} onclick={()=>run('managed_duplicate_import')}>Import separate song</button></div>
  {:else if kind==='delete'}<p>Permanently delete <strong>{songTitle}</strong>, its managed recording, notes, measure map and any stems? This cannot be undone. External originals remain untouched.</p><button disabled={busy} class="button danger" title={keymap.title('managed_confirm_delete')} onclick={()=>run('managed_confirm_delete')}>Delete song permanently</button>{/if}
</dialog>
<style>.heading{display:flex;gap:12px;align-items:center}.heading h2{margin:0}.heading .icon-button{flex-shrink:0}p{margin:20px 0;line-height:1.6}.choices{display:grid;gap:14px}.choices button{text-align:left;white-space:normal}.dialog-error{color:var(--error-text);overflow-wrap:anywhere}.danger{color:var(--error-text);border-color:var(--error-text)}</style>
