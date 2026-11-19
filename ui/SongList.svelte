<script lang="ts">
  // The Library's song list (concept LD, library.md): a single-select listbox
  // and the screen's home focus. A click selects; double-click, Enter or the
  // row's → opens in Practice; ⋯ or ⌘I edits. The loaded song's → returns to
  // Practice as it was left. On touch, a tap opens and a long press edits.
  import Icon from './Icon.svelte';
  import { songLength, libraryDate, type LibraryEntry } from './library';
  import { moveSelection, SONG_LIST_KEYS } from './song-list';
  import type { ButtonId } from './actions';
  import type { ActionPayload } from './action-dispatcher';
  import type { Keymap } from './keymap';
  let { songs, selected, editing, current, keymap, available, run, onselect, onedit }:
    { songs: LibraryEntry[]; selected: string | null; editing: string | null; current: string | null; keymap: Keymap;
      available: (id: ButtonId) => boolean; run: (id: ButtonId, payload?: ActionPayload) => void;
      onselect: (key: string) => void; onedit: (key: string) => void } = $props();
  const ROW_HEIGHT = 46, LONG_PRESS_MS = 500;
  let list: HTMLUListElement;
  const rowId = (key: string) => `song-${key}`;

  function open(key: string) {
    if (key === current) run('library_close'); else run('library_open', { songKey: key });
  }
  function keydown(event: KeyboardEvent) {
    if (event.defaultPrevented || event.altKey || event.ctrlKey || event.metaKey || event.isComposing) return;
    if (event.key === 'Enter') {
      if (selected) { event.preventDefault(); open(selected); }
      return;
    }
    const pane = list.parentElement;
    const page = pane ? Math.floor(pane.clientHeight / ROW_HEIGHT) - 1 : 0;
    const next = moveSelection(songs.map(song => song.song_key), selected, event.key, page || 10);
    if (next === undefined) return;
    event.preventDefault();
    onselect(next);
    requestAnimationFrame(() => document.getElementById(rowId(next))?.scrollIntoView?.({ block: 'nearest' }));
  }

  // Touch: a tap opens, a long press edits. The pointer type of the press
  // decides, since a touch click and a mouse click look the same.
  let touch = false, pressTimer: ReturnType<typeof setTimeout> | undefined, longPressed = false;
  function press(event: PointerEvent, key: string) {
    touch = event.pointerType === 'touch'; longPressed = false; clearTimeout(pressTimer);
    if (touch) pressTimer = setTimeout(() => { longPressed = true; onedit(key); }, LONG_PRESS_MS);
  }
  function release() { clearTimeout(pressTimer); }
  function click(key: string) {
    if (longPressed) { longPressed = false; return; }
    if (touch) open(key); else onselect(key);
  }
</script>

<ul bind:this={list} class="songs" role="listbox" aria-label="Songs" tabindex="0" data-home data-owns-keys={SONG_LIST_KEYS.join(' ')}
  aria-activedescendant={selected ? rowId(selected) : undefined} onkeydown={keydown}>
  {#each songs as song (song.song_key)}
    {@const isCurrent = song.song_key === current}
    {@const title = song.metadata.title}
    <!-- The listbox handles the keys for its options through aria-activedescendant. -->
    <!-- svelte-ignore a11y_click_events_have_key_events -->
    <li id={rowId(song.song_key)} role="option" aria-selected={song.song_key === selected} aria-current={isCurrent ? 'true' : undefined}
      class:current={isCurrent} class:editing={song.song_key === editing} title={`Last opened ${libraryDate(song.last_opened)}`}
      onpointerdown={event => press(event, song.song_key)} onpointerup={release} onpointercancel={release} onpointerleave={release}
      onclick={() => click(song.song_key)} ondblclick={() => open(song.song_key)}>
      <button class="go" tabindex="-1" aria-label={isCurrent ? `Back to ${title}` : `Practise ${title}`}
        disabled={!available(isCurrent ? 'library_close' : 'library_open')} title={keymap.title(isCurrent ? 'library_close' : 'library_open')}
        onclick={event => { event.stopPropagation(); open(song.song_key); }} ondblclick={event => event.stopPropagation()}><Icon name="arrow" size={16}/></button>
      <span class="title"><strong>{title}</strong>{#if isCurrent}<span class="tag">Current</span>{/if}</span>
      <span class="artist" class:unknown={!song.metadata.artist}>{song.metadata.artist || 'Unknown artist'}</span>
      <span class="length">{songLength(song.duration_ms)}</span>
      <button class="more" tabindex="-1" aria-label={`Edit details of ${title}`} title={keymap.title('library_edit')}
        onclick={event => { event.stopPropagation(); onedit(song.song_key); }} ondblclick={event => event.stopPropagation()}><Icon name="more" size={16}/></button>
    </li>
  {/each}
</ul>

<style>
  .songs{flex:0 1 auto;min-height:0;overflow:auto;overscroll-behavior:contain;list-style:none;margin:0;padding:0;border:1px solid var(--border);border-radius:10px;background:var(--surface)}
  .songs:focus-visible{outline:none}
  /* Focused without a selection, the ring goes around the list itself. */
  :global([data-modality=keyboard]) .songs:focus-visible:not([aria-activedescendant]){outline:2px solid var(--focus);outline-offset:1px}
  li{position:relative;display:grid;grid-template-columns:var(--song-columns);align-items:center;gap:12px;min-height:46px;padding:0 10px 0 12px;border-top:1px solid var(--border);background:var(--surface);font-size:13px}
  li:first-child{border-top:0}
  li:hover{background:var(--hover)}
  li[aria-selected=true],li.editing{background:var(--selected)}
  /* Keyboard focus shows on the selected row, not around the list. */
  :global([data-modality=keyboard]) .songs:focus-visible li[aria-selected=true]{outline:2px solid var(--focus);outline-offset:-2px}
  li.current::before{content:'';position:absolute;left:0;top:8px;bottom:8px;width:3px;border-radius:0 2px 2px 0;background:var(--blue)}
  .title{min-width:0;display:flex;align-items:center;gap:8px}
  strong{font-weight:600;white-space:nowrap;overflow:hidden;text-overflow:ellipsis}
  .tag{flex:none;height:18px;display:inline-flex;align-items:center;padding:0 7px;border-radius:9px;background:var(--blue-wash);color:var(--blue);font-size:10px;font-weight:700;letter-spacing:.04em;text-transform:uppercase}
  .artist{white-space:nowrap;overflow:hidden;text-overflow:ellipsis}
  .artist.unknown,.length{color:var(--text-secondary)}
  .length{text-align:right;font-variant-numeric:tabular-nums}
  button{width:30px;height:30px;display:inline-flex;align-items:center;justify-content:center;padding:0;border:0;background:transparent;visibility:hidden}
  .go{border-radius:15px;color:var(--blue)}
  .more{border-radius:6px;color:var(--text-secondary)}
  .more:hover:not(:disabled){background:var(--hover);color:var(--text)}
  /* → and ⋯ show on hover, on the selected and edited rows; the loaded song always shows its →. */
  li:hover button,li[aria-selected=true] button,li.editing button,li.current .go{visibility:visible}
  li:hover .go:not(:disabled),li[aria-selected=true] .go:not(:disabled){background:var(--blue);color:var(--on-accent)}
  @media (pointer:coarse){button{visibility:visible}}
</style>
