<script lang="ts">
  // The Library's dialogs: first-import setup, the one-time copy notice, the
  // duplicate choice and the delete confirmation, plus library settings until
  // app-wide Settings take them over (task 4).
  import Icon from './Icon.svelte';
  import type {ButtonId} from './actions';
  import type {Keymap} from './keymap';
  import {libraryDate,type CatalogueEntry} from './library';
  import type {ManagedDialog} from './managed-library-session';
  let {kind,keymap,run,busy=false,error='',candidates=[],songTitle='',importName='',libraryRoot=null}:{kind:ManagedDialog;keymap:Keymap;busy?:boolean;error?:string;run:(id:ButtonId,songKey?:string)=>void;candidates?:CatalogueEntry[];songTitle?:string;importName?:string;libraryRoot?:string|null}=$props();
  let dialog:HTMLDialogElement;
  // A dialog opens with focus on its default button; deleting defaults to keeping the song.
  $effect(()=>{
    if(kind&&!dialog.open){dialog.showModal();dialog.querySelector<HTMLElement>('[data-default]')?.focus();}
    else if(!kind&&dialog.open)dialog.close();
  });
  const title=$derived(kind==='setup'?'Where should your songs live?':kind==='settings'?'Library settings':kind==='notice'?'Import your music':kind==='duplicates'?'Already in your library':`Delete “${songTitle}”?`);
</script>
<dialog bind:this={dialog} aria-label={title} oncancel={event=>{event.preventDefault();if(!busy)run('managed_close');}}>
  <div class="heading"><button disabled={busy} class="icon-button" aria-label="Close library dialog" title={keymap.title('managed_close')} onclick={()=>run('managed_close')}><Icon name="close" size={16}/></button><h2>{title}</h2></div>
  {#if kind&&error}<p class="dialog-error" role="alert">{error}</p>{/if}
  {#if kind==='setup'||kind==='settings'}
    {#if kind==='setup'}<p>OhMyLoop keeps a copy of each recording in a library folder, next to its notes, measure map and stems. Your original files stay where they are.</p>
    {:else if libraryRoot}<p>Your library is at <code class="copyable">{libraryRoot}</code>. To move or back it up, close OhMyLoop and copy the whole folder, then open it here.</p>
    {:else}<p>You don’t have a library yet. It’s created on your first import, or here.</p>{/if}
    <button disabled={busy} class="choice default" data-default title={keymap.title('managed_create')} onclick={()=>run('managed_create')}><b>Create a library…</b><span>Choose a folder; an OhMyLoop Library folder is made inside it</span></button>
    <button disabled={busy} class="choice" title={keymap.title('managed_open')} onclick={()=>run('managed_open')}><b>Open an existing library…</b><span>A library folder copied from another computer or a backup</span></button>
    {#if kind==='setup'}<div class="actions"><button disabled={busy} class="button" title={keymap.title('managed_close')} onclick={()=>run('managed_close')}>Cancel</button></div>{/if}
  {:else if kind==='notice'}
    <p>Each recording you import is copied into your library, next to its notes and measure map. Your original files stay where they are.</p>
    <div class="actions"><button disabled={busy} class="button primary" data-default title={keymap.title('managed_continue')} onclick={()=>run('managed_continue')}>Continue</button></div>
  {:else if kind==='duplicates'}
    <p>{importName?`“${importName}”`:'This recording'} is the same recording as a song already in your library.</p>
    {#each candidates as entry,index}<button disabled={busy} class="choice" class:default={index===0} data-default={index===0?'':undefined} title={keymap.title('managed_duplicate_open')} onclick={()=>run('managed_duplicate_open',entry.song_key)}><b>Open “{entry.metadata.title}”{entry.metadata.artist?` · ${entry.metadata.artist}`:''}</b><span>Added {libraryDate(entry.date_added)} · with its notes, map and sections</span></button>{/each}
    <button disabled={busy} class="choice" title={keymap.title('managed_duplicate_import')} onclick={()=>run('managed_duplicate_import')}><b>Import as a separate song</b><span>Another copy with its own notes and measure map, e.g. for a different part</span></button>
  {:else if kind==='delete'}
    <p>Its library copy, notes, measure map, sections and stems are deleted permanently. This can’t be undone. The original file stays where it was.</p>
    <div class="actions"><button disabled={busy} class="button" data-default title={keymap.title('managed_close')} onclick={()=>run('managed_close')}>Keep song</button><button disabled={busy} class="button danger" title={keymap.title('managed_confirm_delete')} onclick={()=>run('managed_confirm_delete')}>Delete permanently</button></div>
  {/if}
</dialog>
<style>
  dialog[open]{width:520px;display:flex;flex-direction:column;gap:12px;padding:18px 20px 20px;border-radius:12px}
  .heading{display:flex;gap:8px;align-items:center}.heading h2{margin:0;font-size:16px}.heading .icon-button{flex-shrink:0;margin-left:-6px}
  p{margin:0;line-height:1.55;font-size:13px;color:var(--text)}
  code{font-size:12px;overflow-wrap:anywhere}
  .choice{display:flex;flex-direction:column;gap:3px;text-align:left;padding:12px 14px;border:1px solid var(--border);border-radius:10px;background:var(--surface);color:var(--text);font:inherit;font-size:13px}
  .choice.default{border:2px solid var(--blue);padding:11px 13px;background:var(--blue-wash)}
  .choice:hover:not(:disabled){background:var(--hover)}
  .choice span{font-size:12px;color:var(--text-secondary)}
  .actions{display:flex;justify-content:flex-end;gap:8px;margin-top:4px}
  .actions .button{height:30px;padding:0 12px;font-size:12px;font-weight:600}
  .dialog-error{color:var(--error-text);overflow-wrap:anywhere}
  .danger{border-color:var(--error-text);background:var(--error-text);color:var(--on-accent)}
  .danger:hover:not(:disabled){filter:brightness(1.08);background:var(--error-text)}
</style>
