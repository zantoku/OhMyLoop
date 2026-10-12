<script lang="ts">
  import {onDestroy} from 'svelte';
  import {LibrarySession,type LibraryState} from './library-session';
  import {libraryColumns,librarySongs,libraryDate,type LibraryEntry,type LibrarySort,type SongMetadata} from './library';
  import type {Keymap} from './keymap';
  import type {ActionId,ButtonId} from './actions';
  import type {ActionPayload} from './action-dispatcher';
  let {active,currentKey,keymap,available,run,onchanged}:{active:boolean;currentKey:string|null;keymap:Keymap;available:(id:ButtonId)=>boolean;run:(id:ButtonId,payload?:ActionPayload)=>void;onchanged:(entry:LibraryEntry)=>void}=$props();
  let libraryState=$state<LibraryState>({catalogue:null,pending:false,error:''});
  const session=new LibrarySession(value=>{libraryState=value;});onDestroy(()=>session.dispose());
  $effect(()=>{if(active)void session.refresh();});
  let query=$state(''),sort=$state<LibrarySort>('last_opened'),descending=$state(true);
  let editing=$state<string|null>(null),metadata=$state<SongMetadata>({artist:'',title:'',album:''});
  const songs=$derived(librarySongs(libraryState.catalogue?.songs??[],query,sort,descending));
  const selected=$derived(libraryState.catalogue?.songs.find(entry=>entry.song_key===editing)??null);
  export function canRun(id:ActionId){
    if(id==='library_cancel')return !!editing&&!libraryState.pending;
    if(id==='library_save')return !!selected&&!libraryState.pending&&!!metadata.title.trim();
    if(id==='library_retry')return !libraryState.pending;
    if(id==='library_sort'||id==='library_reverse')return true;
    return !!libraryState.catalogue&&!libraryState.pending;
  }
  export async function command(id:ActionId,payload:ActionPayload={}){
    if(!canRun(id))return;
    if(id==='library_edit'){
      const entry=libraryState.catalogue?.songs.find(entry=>entry.song_key===payload.songKey);
      if(entry){editing=entry.song_key;metadata={...entry.metadata};}
    }
    if(id==='library_cancel')editing=null;
    if(id==='library_retry')await session.refresh();
    if(id==='library_reverse')descending=!descending;
    if(id==='library_sort'&&payload.librarySort){if(sort===payload.librarySort)descending=!descending;else{sort=payload.librarySort;descending=sort==='date_added'||sort==='last_opened';}}
    if(id==='library_save'&&selected){const entry=await session.save(selected.song_key,$state.snapshot(metadata));if(entry){onchanged(entry);editing=null;}}
  }
  const disabled=(id:ButtonId)=>!active||!available(id);
</script>
{#if active}
<main class="library-screen" aria-label="Music library" aria-busy={libraryState.pending}>
  <div class="library-heading"><div><h2>Your library</h2><p>{libraryState.catalogue?.songs.length??0} {(libraryState.catalogue?.songs.length??0)===1?'song':'songs'} to come back to</p></div><button class="button" disabled={disabled('open_song')} title={keymap.title('open_song')} onclick={()=>run('open_song')}>{keymap.buttonLabel('open_song','Open song')}</button></div>
  <p><button class="text-button" disabled={disabled('import_separate')} title={keymap.title('import_separate')} onclick={()=>run('import_separate')}>Import separate song</button> creates independent notes and maps, even for an exact copy. Open song reopens an existing exact recording.</p>
  <div class="library-toolbar"><label class="search">Find a song<input type="search" aria-label="Search library" bind:value={query} placeholder="Title, artist, album or file location"/></label><label>Sort by<select aria-label="Sort library by" bind:value={sort} onchange={()=>{descending=sort==='date_added'||sort==='last_opened';}}>{#each libraryColumns as column}<option value={column.field}>{column.label}</option>{/each}</select></label><button class="text-button direction" disabled={disabled('library_reverse')} title={keymap.title('library_reverse')} onclick={()=>run('library_reverse')}>{keymap.buttonLabel('library_reverse',descending?'Descending ↓':'Ascending ↑')}</button></div>
  {#if libraryState.error}<div class="library-error" role="alert"><span>{libraryState.error}</span><button class="text-button" disabled={disabled('library_retry')} title={keymap.title('library_retry')} onclick={()=>run('library_retry')}>{keymap.buttonLabel('library_retry','Retry')}</button></div>{/if}
  <div class="library-content" class:with-editor={!!selected}>
    <div class="library-list">
      {#if libraryState.pending&&!libraryState.catalogue}<p class="empty" role="status">Loading library…</p>
      {:else if !libraryState.catalogue?.songs.length&&!libraryState.error}<div class="empty"><h3>Keep your practice songs here.</h3><p>Open a song to add it. Its sections and measure map will be ready next time.</p></div>
      {:else if !songs.length&&!libraryState.error}<p class="empty">No songs match your search.</p>
      {:else}<table><caption class="sr-only">Practice songs</caption><thead><tr>{#each libraryColumns as column}<th aria-sort={sort===column.field?(descending?'descending':'ascending'):'none'}><button title={keymap.title('library_sort')} onclick={()=>run('library_sort',{librarySort:column.field})}>{column.label}{sort===column.field?(descending?' ↓':' ↑'):''}</button></th>{/each}<th><span class="sr-only">Song actions</span></th></tr></thead><tbody>{#each songs as entry(entry.song_key)}<tr class:current={entry.song_key===currentKey} class:editing={entry.song_key===editing}><td class="song-cell"><button class="song-title" disabled={disabled('library_open')} title={keymap.title('library_open')} onclick={()=>run('library_open',{songKey:entry.song_key})}>{entry.metadata.title}</button>{#if entry.song_key===currentKey}<span class="current-label">Current song</span>{/if}</td><td>{entry.metadata.artist||'—'}</td><td>{entry.metadata.album||'—'}</td><td class="location" title={entry.file_location}>{entry.file_location}</td><td class="date"><time datetime={new Date(entry.date_added).toISOString()}>{libraryDate(entry.date_added)}</time></td><td class="date"><time datetime={new Date(entry.last_opened).toISOString()}>{libraryDate(entry.last_opened)}</time></td><td><button class="text-button" aria-label={`Edit details for ${entry.metadata.title}`} disabled={disabled('library_edit')} title={keymap.title('library_edit')} onclick={()=>run('library_edit',{songKey:entry.song_key})}>{keymap.buttonLabel('library_edit','Edit')}</button><button class="text-button" aria-label={`Replace recording for ${entry.metadata.title}`} disabled={disabled('library_replace')} title={keymap.title('library_replace')} onclick={()=>run('library_replace',{songKey:entry.song_key})}>{keymap.buttonLabel('library_replace','Replace recording')}</button></td></tr>{/each}</tbody></table>{/if}
    </div>
    {#if selected}<aside class="library-details" aria-label="Song details"><h3>Song details</h3><form onsubmit={event=>{event.preventDefault();run('library_save');}}><label>Title<input aria-label="Library song title" bind:value={metadata.title} required maxlength="1024"/></label><label>Artist<input aria-label="Library song artist" bind:value={metadata.artist} maxlength="1024"/></label><label>Album<input aria-label="Library song album" bind:value={metadata.album} maxlength="1024"/></label><div class="details-actions"><button class="button primary" disabled={disabled('library_save')} title={keymap.title('library_save')}>{keymap.buttonLabel('library_save','Save details')}</button><button class="text-button" type="button" disabled={disabled('library_cancel')} title={keymap.title('library_cancel')} onclick={()=>run('library_cancel')}>{keymap.buttonLabel('library_cancel','Cancel')}</button></div></form><p class="details-note">These details are saved in your library. Your audio file stays unchanged.</p><dl><dt>File location</dt><dd class="detail-location">{selected.file_location}</dd><dt>Date added</dt><dd>{libraryDate(selected.date_added)}</dd><dt>Last opened</dt><dd>{libraryDate(selected.last_opened)}</dd></dl></aside>{/if}
  </div>
</main>
{/if}
<style>
  .library-screen{display:flex;flex-direction:column;flex:1;min-height:0;background:var(--surface);border:1px solid var(--border);border-radius:9px;overflow:hidden}.library-heading{display:flex;align-items:center;justify-content:space-between;padding:22px 25px 16px;gap:16px}.library-heading h2{font-size:23px;font-weight:500;letter-spacing:-.5px}.library-heading p{font-size:12px;color:var(--muted);margin-top:4px}.library-toolbar{display:flex;gap:16px;align-items:end;padding:0 25px 20px;border-bottom:1px solid var(--border)}.library-toolbar .search{flex:1}.library-toolbar label{min-width:150px}.direction{white-space:nowrap;margin-bottom:5px}.library-content{display:flex;flex:1;min-height:0}.library-list{flex:1;min-width:0;overflow:auto}.library-details{width:290px;flex-shrink:0;padding:22px 20px;border-left:1px solid var(--border);background:var(--sidebar);overflow:auto}label{display:flex;flex-direction:column;gap:6px;font-size:11px;color:var(--text-secondary)}input,select{min-width:0;width:100%;padding:8px 10px;background:var(--surface);border:1px solid var(--border-strong);border-radius:4px;font:inherit;font-size:13px;color:var(--text)}.library-details label{margin-top:16px}.details-actions{display:flex;gap:7px;margin-top:20px;align-items:center}.details-actions .button{font-size:12px}.details-note{font-size:11px;color:var(--muted);line-height:1.6;margin-top:15px}dl{font-size:11px;line-height:1.6;margin-top:24px}dt{color:var(--muted);margin-top:14px}dd{margin:4px 0}.detail-location{overflow-wrap:anywhere}table{width:100%;border-collapse:collapse;font-size:12px;text-align:left}thead{position:sticky;top:0;background:var(--sidebar);z-index:1}th{font-size:11px;font-weight:500;white-space:nowrap;border-bottom:1px solid var(--border)}th button{border:0;background:transparent;padding:14px 16px;width:100%;text-align:left;color:var(--text-secondary)}td{padding:17px 16px;border-bottom:1px solid var(--border);vertical-align:top}.song-cell{min-width:180px}.song-title{border:0;background:transparent;padding:0;text-align:left;font-size:13px;font-weight:600;color:var(--text)}.song-title:hover{text-decoration:underline}.current-label{display:block;font-size:10px;color:var(--success-text);margin-top:5px}.current{background:var(--success-bg)}.editing{background:var(--hover)}.location{max-width:240px;min-width:160px;overflow-wrap:anywhere;color:var(--muted);font-size:11px}.date{min-width:140px;color:var(--muted);font-size:11px}.empty{padding:70px 25px;text-align:center;color:var(--muted);font-size:13px}.empty h3{font-size:20px;font-weight:500;color:var(--text)}.empty p{margin:10px auto 22px;max-width:45ch}.library-error{display:flex;gap:15px;align-items:center;justify-content:space-between;padding:12px 25px;background:var(--error-bg);color:var(--error-text);font-size:12px}.library-error span{overflow-wrap:anywhere}.sr-only{position:absolute;width:1px;height:1px;padding:0;margin:-1px;overflow:hidden;clip:rect(0,0,0,0);white-space:nowrap;border:0}
  @media(max-width:1000px){.library-heading,.library-toolbar{padding-left:17px;padding-right:17px}.library-details{width:250px;padding:18px 15px}.library-toolbar{gap:10px}.library-toolbar label{min-width:120px}}
</style>
