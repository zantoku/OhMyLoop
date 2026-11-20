<script lang="ts">
  // The Library (concept LD, docs/design/library.md): find and open a song,
  // import, and edit a song's details only through an explicit edit action.
  // Opening a song never shows its edit fields.
  import Icon from './Icon.svelte';
  import SongList from './SongList.svelte';
  import SongEditor from './SongEditor.svelte';
  import type {LibraryStatus} from './types';
  import {onDestroy,tick} from 'svelte';
  import {LibrarySession,type LibraryState} from './library-session';
  import {librarySorts,librarySongs,sortsDescending,type LibraryEntry,type LibrarySort,type SongMetadata} from './library';
  import {focusHome} from './home-focus';
  import {openTransient} from './transient';
  import type {Keymap} from './keymap';
  import type {ActionId,ButtonId} from './actions';
  import type {ActionPayload} from './action-dispatcher';
  let {active,currentKey,keymap,available,run,onchanged,status,epoch=0}:{status:LibraryStatus;epoch?:number;active:boolean;currentKey:string|null;keymap:Keymap;available:(id:ButtonId)=>boolean;run:(id:ButtonId,payload?:ActionPayload)=>void;onchanged:(entry:LibraryEntry)=>void}=$props();
  let libraryState=$state<LibraryState>({catalogue:null,pending:false,error:'',saveError:''});
  const session=new LibrarySession(value=>{libraryState=value;});
  let query=$state(''),sort=$state<LibrarySort>('last_opened'),descending=$state(true);
  let selectedKey=$state<string|null>(null),editingKey=$state<string|null>(null),metadata=$state<SongMetadata>({artist:'',title:'',album:''});
  let searchField=$state<HTMLInputElement>(),editor=$state<SongEditor>();
  const songs=$derived(librarySongs(libraryState.catalogue?.songs??[],query,sort,descending));
  const entryFor=(key:string|null)=>libraryState.catalogue?.songs.find(entry=>entry.song_key===key)??null;
  const editing=$derived(entryFor(editingKey));
  const missing=$derived(!!status.selected&&!status.available);
  const total=$derived(libraryState.catalogue?.songs.length??0);
  const dirty=$derived(!!editing&&(metadata.title!==editing.metadata.title||metadata.artist!==editing.metadata.artist||metadata.album!==editing.metadata.album));

  // The open editor is one Esc level: Esc at home closes it before leaving the Library.
  let releaseEditor:(()=>void)|null=null;
  function closeEditor(){releaseEditor?.();releaseEditor=null;editingKey=null;session.clearSaveError();}
  /** Shows a song in the editor; only an explicit edit moves focus into its Title. */
  async function startEditing(key:string,focus:boolean){
    const entry=entryFor(key);if(!entry)return;
    editingKey=key;metadata={...entry.metadata};session.clearSaveError();
    releaseEditor??=openTransient(()=>run('library_cancel'));
    if(focus){await tick();editor?.focusTitle();}
  }
  // Leaving the Library saves the details first (App flushes before every
  // exit), so the editor stays as it was; only its Esc level is released.
  let lastEpoch=-1;
  $effect(()=>{
    if(!active){releaseEditor?.();releaseEditor=null;return;}
    if(epoch!==lastEpoch){lastEpoch=epoch;session.invalidate();closeEditor();selectedKey=null;}
    void session.refresh();
  });
  $effect(()=>{if(active&&editingKey&&!releaseEditor)releaseEditor=openTransient(()=>run('library_cancel'));});
  onDestroy(()=>{releaseEditor?.();session.dispose();});

  /** Selecting another song while editing saves pending edits first; the editor follows. */
  async function select(key:string){
    if(editingKey&&editingKey!==key){if(!await flush())return false;selectedKey=key;await startEditing(key,false);return true;}
    selectedKey=key;return true;
  }
  async function edit(key:string){if(await select(key))run('library_edit');}

  export function canRun(id:ActionId){
    if(id==='managed_delete'||id==='library_replace')return !!editing&&!libraryState.pending;
    if(id==='library_edit')return !!entryFor(selectedKey)&&!libraryState.pending;
    if(id==='library_cancel')return !!editingKey&&!libraryState.pending;
    if(id==='library_save')return !!editing&&!libraryState.pending&&!!metadata.title.trim();
    if(id==='library_retry')return !libraryState.pending;
    if(id==='library_clear_search')return !!query;
    if(id==='library_sort'||id==='library_reverse'||id==='library_find')return true;
    return !!libraryState.catalogue&&!libraryState.pending;
  }
  /** Saves pending detail edits; false when they couldn't be saved and are kept. */
  export async function flush(){
    if(!editing||!dirty)return true;
    if(libraryState.pending)return false;
    const entry=await session.save(editing.song_key,$state.snapshot(metadata));
    if(entry){onchanged(entry);metadata={...entry.metadata};}
    return !!entry;
  }
  export function titleFor(key:string){return entryFor(key)?.metadata.title??'this song';}
  export async function command(id:ActionId,payload:ActionPayload={}){
    if(!canRun(id))return;
    if(id==='library_edit'&&selectedKey)await startEditing(selectedKey,true);
    if(id==='library_cancel'){closeEditor();focusHome();}
    if(id==='library_save'&&await flush()){closeEditor();focusHome();}
    if(id==='library_retry')await session.refresh();
    if(id==='library_find'){searchField?.focus();searchField?.select();}
    if(id==='library_clear_search'){query='';focusHome();}
    if(id==='library_reverse')descending=!descending;
    if(id==='library_sort'&&payload.librarySort){if(sort===payload.librarySort)descending=!descending;else{sort=payload.librarySort;descending=sortsDescending(sort);}}
  }
  const disabled=(id:ButtonId)=>!active||!available(id);
  const editShortcut=$derived(keymap.shortcut('library_edit'));
</script>
{#if active}
<main class="library" aria-label="Music library" aria-busy={libraryState.pending}>
  <div class="catalogue">
    <div class="toolbar">
      <h2>Your songs{#if total}<span class="count">{total} {total===1?'song':'songs'}</span>{/if}</h2>
      <label class="search"><span class="search-icon"><Icon name="search" size={15}/></span>
        <input bind:this={searchField} type="search" aria-label="Find a song" title={keymap.title('library_find')} bind:value={query}
          placeholder={`Title, artist, album or file${keymap.shortcut('library_find')?` (${keymap.shortcut('library_find')})`:''}`}/></label>
      <label class="sort"><Icon name="sort" size={14}/><span class="sr-only">Sort by</span>
        <select aria-label="Sort by" title={keymap.title('library_sort')} value={sort} disabled={disabled('library_sort')}
          onchange={event=>run('library_sort',{librarySort:event.currentTarget.value as LibrarySort})}>{#each librarySorts as option}<option value={option.field}>{option.label}</option>{/each}</select></label>
      <button class="direction" aria-label={descending?'Descending order':'Ascending order'} disabled={disabled('library_reverse')} title={keymap.title('library_reverse')} onclick={()=>run('library_reverse')}>{descending?'↓':'↑'}</button>
      <button class="button primary import" disabled={disabled('open_song')} title={`${keymap.title('open_song')}, or drop an audio file anywhere`} onclick={()=>run('open_song')}><Icon name="plus" size={15}/>{keymap.buttonLabel('open_song','Import song')}</button>
    </div>
    <div class="pane">
      {#if missing}
        <div class="notice missing" role="alert">
          <strong><Icon name="warning" size={16}/>Your library can’t be found</strong>
          <p>Your library was at <code class="copyable">{status.selected?.root}</code>. If it’s on a drive, connect it and retry. If you moved the folder, locate it.</p>
          {#if status.error}<p class="detail copyable">{status.error}</p>{/if}
          <div class="buttons">
            <button class="button primary" disabled={disabled('managed_retry')} title={keymap.title('managed_retry')} onclick={()=>run('managed_retry')}>Retry</button>
            <button class="button" disabled={disabled('managed_locate')} title={keymap.title('managed_locate')} onclick={()=>run('managed_locate')}>Locate library…</button>
            <button class="button ghost" disabled={disabled('managed_open')} title={keymap.title('managed_open')} onclick={()=>run('managed_open')}>Open another library…</button>
          </div>
        </div>
      {:else if libraryState.error}
        <div class="notice failed" role="alert"><Icon name="warning" size={16}/><span class="copyable">Couldn’t read the song list: {libraryState.error}</span>
          <button class="button" disabled={disabled('library_retry')} title={keymap.title('library_retry')} onclick={()=>run('library_retry')}>{keymap.buttonLabel('library_retry','Reload')}</button></div>
      {:else if libraryState.pending&&!libraryState.catalogue}<p class="message" role="status">Loading your songs…</p>
      {:else if !total}
        <div class="empty">
          <span class="music"><Icon name="music" size={30}/></span>
          <h3>Bring in the songs you’re working on</h3>
          <p>Import a recording, or drop an audio file here. It’s copied into your library; the original stays where it is. Its sections, measure map and notes will be ready next time.</p>
          <button class="button primary" disabled={disabled('open_song')} title={keymap.title('open_song')} onclick={()=>run('open_song')}><Icon name="plus" size={15}/>{keymap.buttonLabel('open_song','Import song')}</button>
          <span class="formats">WAV, MP3, FLAC, M4A, AAC, AIFF and Ogg</span>
        </div>
      {:else if !songs.length}
        <div class="message"><span>No songs match “{query.trim()}”.</span><button class="button ghost" disabled={disabled('library_clear_search')} title={keymap.title('library_clear_search')} onclick={()=>run('library_clear_search')}>Clear search</button></div>
      {:else}
        <div class="columns" aria-hidden="true"><span></span><span>Title</span><span>Artist</span><span class="right">Length</span><span></span></div>
        <SongList {songs} selected={selectedKey} editing={editingKey} current={currentKey} {keymap} {available} {run} onselect={key=>{void select(key);}} onedit={key=>{void edit(key);}}/>
        <p class="hint fine">Double-click a song or press Enter to practise it · ⋯{editShortcut?` or ${editShortcut}`:''} edits its details</p>
        <p class="hint coarse">Tap a song to practise it · ⋯ or a long press edits its details</p>
      {/if}
    </div>
  </div>
  {#if editing}<SongEditor bind:this={editor} entry={editing} bind:metadata saveError={libraryState.saveError} {keymap} {available} {run}/>{/if}
</main>
{/if}
<style>
  .library{flex:1;min-height:0;display:flex;background:var(--page)}
  .catalogue{flex:1;min-width:0;display:flex;flex-direction:column;--song-columns:32px minmax(0,1.4fr) minmax(0,1fr) 64px 32px}
  .toolbar{flex:none;display:flex;align-items:center;gap:10px;padding:16px 24px 12px}
  h2{margin:0 12px 0 0;font-size:20px;font-weight:600;letter-spacing:-.3px;white-space:nowrap}
  .count{margin-left:8px;font-size:13px;font-weight:400;color:var(--text-secondary);letter-spacing:0}
  .search{flex:1;min-width:180px;max-width:380px;position:relative;display:flex;align-items:center}
  .search-icon{position:absolute;left:10px;display:flex;color:var(--text-secondary);pointer-events:none}
  input,select{height:34px;border:1px solid var(--border-strong);border-radius:6px;background:var(--surface);color:var(--text);font:inherit;font-size:13px}
  input{width:100%;padding:0 8px 0 32px}
  .sort{display:flex;align-items:center;gap:6px;color:var(--text-secondary)}
  select{padding:0 8px}
  .direction{width:34px;height:34px;border:1px solid var(--border);border-radius:6px;background:var(--surface);color:var(--text-secondary);font-size:14px}
  .direction:hover:not(:disabled){background:var(--hover);color:var(--text)}
  .import{margin-left:auto}
  .button{height:34px;padding:0 14px;font-size:12px;font-weight:600}
  .button.ghost{border-color:transparent;background:transparent;color:var(--link)}
  .button.ghost:hover:not(:disabled){background:var(--hover)}
  /* The song list scrolls; the column header and the hint stay put. */
  .pane{flex:1;min-height:0;overflow:auto;overscroll-behavior:contain;padding:0 24px 24px;display:flex;flex-direction:column}
  .columns,.hint{flex:none}
  .columns{display:grid;grid-template-columns:var(--song-columns);gap:12px;padding:0 10px 6px 12px;font-size:11px;font-weight:700;letter-spacing:.06em;text-transform:uppercase;color:var(--text-secondary)}
  .right{text-align:right}
  .hint{margin:10px 0 0;font-size:11px;line-height:1.45;color:var(--text-secondary)}
  .coarse{display:none}
  @media (pointer:coarse){.coarse{display:block}.fine{display:none}}
  .message{margin:40px 0;display:flex;flex-direction:column;align-items:center;gap:8px;text-align:center;color:var(--text-secondary)}
  .empty{margin:8px 0 0;flex:1;min-height:320px;display:flex;flex-direction:column;align-items:center;justify-content:center;gap:12px;border:2px dashed var(--border-strong);border-radius:14px;color:var(--text-secondary);text-align:center}
  .music{display:flex;color:var(--blue)}
  .empty h3{margin:0;font-size:20px;color:var(--text)}
  .empty p{margin:0;max-width:420px;line-height:1.5}
  .empty .button{height:36px;padding:0 16px;margin-top:6px}
  .formats{font-size:11px}
  .notice{margin:24px auto;max-width:560px;border-radius:12px;text-align:left}
  .missing .button:not(.primary,.ghost){color:var(--text)}
  .missing{display:flex;flex-direction:column;gap:12px;padding:20px 22px;border:1px solid var(--amber-border);background:var(--amber-bg);color:var(--amber-text)}
  .missing strong{display:flex;align-items:center;gap:8px;font-size:15px}
  .missing p{margin:0;line-height:1.5;color:var(--text)}
  .missing code{font-size:12px;overflow-wrap:anywhere}
  .missing .detail{font-size:12px;color:var(--text-secondary)}
  .buttons{display:flex;gap:8px}
  .failed{display:flex;align-items:center;gap:12px;padding:14px 16px;border:1px solid var(--error-border);border-radius:10px;background:var(--error-bg);color:var(--error-text);font-size:13px}
  .failed span{flex:1;overflow-wrap:anywhere}
  .failed :global(svg){flex:none}
</style>
