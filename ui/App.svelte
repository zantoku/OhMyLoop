<script lang="ts">
  import { onMount } from 'svelte';
  import Waveform from './Waveform.svelte';
  import Icon from './Icon.svelte';
  import { bridge, native } from './bridge';
  import { clamp, editing, ordered, range, sectionColor, time, zoom } from './model';
  import type { Action, Loop, PracticeDocument, Section, Snapshot, Song, Transport, View } from './types';

  let song = $state<Song | null>(null);
  let project = $state<PracticeDocument | null>(null);
  let transport = $state<Transport>({ song_key: null, position: 0, playing: false, finished: false, audio_error: null, save_error: null, device: null });
  let busy = $state('');
  let error = $state('');
  let saved = $state(true);
  let view = $state<View>({ start: 0, end: 12 });
  let sectionDialog: HTMLDialogElement;
  let helpDialog: HTMLDialogElement;
  let nameInput: HTMLInputElement;
  let sectionId = $state<string | null>(null);
  let sectionName = $state('');
  let sectionStart = $state(0);
  let sectionEnd = $state(0);
  let dialogError = $state('');
  let revision = 0;
  let saves: Promise<void> = Promise.resolve();
  let controlTimer: ReturnType<typeof setTimeout> | undefined;
  let undo = $state<{ section: Section; selected: boolean }[]>([]);
  let mounted = false;
  const sections = $derived(ordered(project?.sections ?? []));
  const selection = $derived(project?.loop_selection ?? null);
  const speed = $derived(project?.controls.speed ?? 1);
  const pitch = $derived(project?.controls.semitones ?? 0);

  function apply(snapshot: Snapshot, documentRevision = revision) {
    if (snapshot.song) { song = snapshot.song; view = { start: 0, end: snapshot.song.duration }; }
    if (documentRevision === revision) project = snapshot.document;
    if (documentRevision === revision) transport = snapshot.transport;
  }
  function flushControls() {
    if (controlTimer && project) {
      clearTimeout(controlTimer); controlTimer = undefined;
      void update({ controls: project.controls });
    }
  }
  async function open(demo = false) {
    if (busy) return;
    busy = demo ? 'Loading demo…' : 'Opening song…'; error = '';
    try {
      flushControls(); await saves;
      const snapshot = demo ? await bridge.loadDemo() : await bridge.openSong();
      if (snapshot) { revision++; apply(snapshot); saved = true; undo = []; }
    } catch (e) { error = String(e); }
    finally { busy = ''; }
  }
  function update(change: Partial<PracticeDocument>): Promise<void> {
    if (!project) return Promise.resolve();
    project = { ...project, ...change }; const document = $state.snapshot(project);
    const thisRevision = ++revision; saved = false;
    saves = saves.catch(() => {}).then(async () => {
      try {
        const snapshot = await bridge.update(document); apply(snapshot, thisRevision);
        if (thisRevision === revision) saved = true;
      } catch (e) { error = `Changes could not be saved: ${String(e)}`; }
    });
    return saves;
  }
  async function action(action: Action) {
    if (!song || busy) return;
    try { flushControls(); await saves; const actionRevision = revision; apply(await bridge.action(action), actionRevision); }
    catch (e) { error = String(e); }
  }
  function controls(nextSpeed: number, nextPitch: number) {
    if (!project || !Number.isFinite(nextSpeed) || !Number.isFinite(nextPitch)) return;
    project = { ...project, controls: { speed: clamp(nextSpeed, .25, 2), semitones: clamp(nextPitch, -12, 12) } };
    revision++;
    clearTimeout(controlTimer);
    saved = false;
    controlTimer = setTimeout(() => { controlTimer = undefined; void update({ controls: project!.controls }); }, 90);
  }
  async function chooseLoop(loop: Loop) { await update({ loop_selection: loop, loop_enabled: true }); }
  function clearLoop() { return update({ loop_selection: null, loop_enabled: false }); }
  async function toggleLoop() {
    if (!project || !song) return;
    if (!selection) {
      const start = Math.min(transport.position, Math.max(0, song.duration - .1));
      await update({ loop_selection: { start, end: Math.min(song.duration, start + 4), fade_ms: 5 }, loop_enabled: true });
    } else await update({ loop_enabled: !project.loop_enabled });
  }
  async function selectSection(section: Section) {
    const loop = { start: section.start, end: section.end, fade_ms: 5 };
    await update({ selected_section: section.id, loop_selection: loop });
    await action({ type: 'seek', seconds: section.start });
  }
  function nextSection(direction: number) {
    if (!sections.length) return;
    const index = sections.findIndex(s => s.id === project?.selected_section);
    const selected = index < 0 ? (direction > 0 ? 0 : sections.length - 1) : clamp(index + direction, 0, sections.length - 1);
    void selectSection(sections[selected]);
  }
  function startSection(section?: Section) {
    if (!song) return;
    sectionId = section?.id ?? null; sectionName = section?.name ?? '';
    sectionStart = section?.start ?? selection?.start ?? Math.min(transport.position, song.duration - .1);
    sectionEnd = section?.end ?? selection?.end ?? song.duration;
    dialogError = ''; sectionDialog.showModal(); requestAnimationFrame(() => nameInput?.focus());
  }
  async function saveSection(event: SubmitEvent) {
    event.preventDefault(); if (!song || !project) return;
    const selected = range(sectionStart, sectionEnd, song.duration);
    if (!sectionName.trim() || sectionName.length > 128) { dialogError = 'Give this section a name (up to 128 characters).'; return; }
    if (!selected || sectionStart >= sectionEnd || sectionStart < 0 || sectionEnd > song.duration) { dialogError = 'Choose a start and end inside the song, at least 0.1 seconds apart.'; return; }
    const section: Section = { id: sectionId ?? crypto.randomUUID(), name: sectionName.trim(), start: sectionStart, end: sectionEnd };
    const next = sectionId ? project.sections.map(s => s.id === sectionId ? section : s) : [...project.sections, section];
    await update({ sections: next, selected_section: section.id }); sectionDialog.close();
  }
  async function removeSection(id: string) {
    if (!project) return;
    const section = project.sections.find(s => s.id === id);
    if (section) undo = [...undo, { section, selected: project.selected_section === id }];
    await update({ sections: project.sections.filter(s => s.id !== id), selected_section: project.selected_section === id ? null : project.selected_section });
  }
  async function undoDelete() {
    const last = undo.at(-1); if (!last || !project) return;
    undo = undo.slice(0, -1);
    await update({ sections: [...project.sections, last.section], selected_section: last.selected && !project.selected_section ? last.section.id : project.selected_section });
  }
  function fitLoop() { if (selection && song) { const pad = (selection.end - selection.start) * .15; view = { start: Math.max(0, selection.start - pad), end: Math.min(song.duration, selection.end + pad) }; } }
  function setBoundary(edge: 'start' | 'end') {
    if (!song) return;
    const start = edge === 'start' ? transport.position : selection?.start ?? 0;
    const end = edge === 'end' ? transport.position : selection?.end ?? Math.min(song.duration, transport.position + 4);
    const loop = range(start, end, song.duration); if (loop) void chooseLoop(loop);
  }
  function keyboard(event: KeyboardEvent) {
    if (editing(event.target) || busy || sectionDialog?.open || helpDialog?.open) return;
    if ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === 'o') { event.preventDefault(); void open(); return; }
    if (!song || event.metaKey || event.ctrlKey || event.altKey) return;
    const key = event.key.toLowerCase();
    if (event.code === 'Space') { event.preventDefault(); if (native && !transport.audio_error) void action({ type: 'toggle_play' }); }
    else if (key === 'l') { event.preventDefault(); void toggleLoop(); }
    else if (key === 'x') { event.preventDefault(); void clearLoop(); }
    else if (key === 'a' || key === 'b') { event.preventDefault(); setBoundary(key === 'a' ? 'start' : 'end'); }
    else if (key === '[' || key === ']') { event.preventDefault(); nextSection(key === '[' ? -1 : 1); }
    else if (key === 'n') { event.preventDefault(); startSection(); }
    else if (key === 'arrowleft' || key === 'arrowright') { event.preventDefault(); void action({ type: 'seek', seconds: clamp(transport.position + (key === 'arrowright' ? 1 : -1) * (event.shiftKey ? .1 : 5), 0, song.duration) }); }
    else if (key === 'escape' && project?.loop_enabled) void update({ loop_enabled: false });
  }
  onMount(() => {
    mounted = true; busy = 'Restoring practice…';
    bridge.initialize().then(apply).catch(e => { error = String(e); }).finally(() => { busy = ''; });
    let polling = false;
    const timer = setInterval(async () => {
      if (!song || busy || polling) return;
      polling = true;
      try { const status = await bridge.status(); if (mounted && status.song_key === song?.key) transport = status; }
      catch (e) { if (mounted) error = String(e); } finally { polling = false; }
    }, 50);
    return () => { mounted = false; clearInterval(timer); clearTimeout(controlTimer); };
  });
</script>

<svelte:window onkeydown={keyboard}/>
<div class="app-shell">
  <header class="app-header">
    <div class="brand" aria-label="OhMyLoop">
      <svg viewBox="0 0 34 34" width="34" height="34" aria-hidden="true"><path d="M9 8a12 12 0 1 1-4 13" fill="none" stroke="#526fc0" stroke-width="3" stroke-linecap="round"/><path d="m5 6 4 2-1 5" fill="none" stroke="#526fc0" stroke-width="3" stroke-linecap="round"/><path d="M11 17h2l2-5 3 10 3-6h2" fill="none" stroke="#bd8941" stroke-width="2" stroke-linecap="round"/></svg>
      <span>OhMyLoop</span>
    </div>
    <div class="song-heading"><h1>{song?.title ?? 'Make room for practice'}</h1><span>{song ? `${time(song.duration, 0)} • ${song.path ? 'Local song' : 'Built-in demo'}` : 'One song. As many tries as it takes.'}</span></div>
    <button class="button open-button" onclick={() => open()} disabled={!!busy}><Icon name="folder"/>Open song</button>
    <button class="icon-button" aria-label="Keyboard shortcuts" title="Keyboard shortcuts" onclick={() => helpDialog.showModal()}><Icon name="help"/></button>
  </header>

  {#if error}<div class="notice error" role="alert"><span>{error}</span><button class="text-button" onclick={() => { error = ''; }}>Dismiss</button></div>{/if}
  {#if transport.audio_error && song}<div class="notice audio-notice" role="status"><span>Audio output is unavailable. Reconnect your output device, then retry.</span><button class="text-button" onclick={() => action({ type: 'retry_audio' })}>Retry audio</button></div>{/if}
  {#if transport.save_error}<div class="notice error" role="alert"><span>{transport.save_error}</span><button class="text-button" onclick={() => update({})}>Retry save</button></div>{/if}
  {#if !native}<div class="preview-note">Interface preview · Audio playback and file opening are available in the desktop app.</div>{/if}

  <main class="practice-layout" aria-busy={!!busy}>
    <aside class="sections-panel">
      <div class="panel-title"><h2>Sections</h2><span>{sections.length || ''}</span></div>
      <p class="panel-intro">Find your place. Work on what needs it.</p>
      {#if sections.length}
        <div class="section-list">
          {#each sections as section (section.id)}
            <div class:selected={project?.selected_section === section.id} class="section-row" style:--section-color={sectionColor(section.id)}>
              <button class="section-select" onclick={() => selectSection(section)} aria-pressed={project?.selected_section === section.id}>
                <span class="section-dot"></span><span class="section-text"><strong>{section.name}</strong><span>{time(section.start)} – {time(section.end)}</span></span><span class="section-duration">{time(section.end - section.start, 0)}</span>
              </button>
              <div class="section-actions"><button class="icon-button" aria-label={`Edit ${section.name}`} onclick={() => startSection(section)}><Icon name="edit" size={15}/></button><button class="icon-button" aria-label={`Delete ${section.name}`} onclick={() => removeSection(section.id)}><Icon name="trash" size={15}/></button></div>
            </div>
          {/each}
        </div>
      {:else}<div class="sections-empty">{song ? 'Save a passage as a section to jump back to it.' : 'Your song’s sections will live here.'}</div>{/if}
      <button class="button add-section" disabled={!song || !!busy} onclick={() => startSection()}><Icon name="plus" size={17}/>Add section</button>
      <div class="sidebar-tip"><span class="keycap">[</span><span class="keycap">]</span><span>Move between sections</span></div>
    </aside>

    <section class="workbench" aria-label="Practice workspace">
      {#if song && project}
        <div class="workspace-heading"><div><h2>Your practice space</h2><p>Click to seek. Drag to loop. Scroll to zoom; middle-drag to pan.</p></div><div class="zoom-controls"><button class="icon-button" aria-label="Zoom out" disabled={view.end - view.start >= song.duration} onclick={() => { view = zoom(view, song!.duration, 2, (view.start + view.end) / 2); }}><Icon name="minus" size={17}/></button><span>{Math.round(song.duration / (view.end - view.start))}×</span><button class="icon-button" aria-label="Zoom in" onclick={() => { view = zoom(view, song!.duration, .5, selection ? (selection.start + selection.end) / 2 : transport.position); }}><Icon name="plus" size={17}/></button><button class="icon-button" aria-label="Show whole song" title="Show whole song" onclick={() => { view = { start: 0, end: song!.duration }; }}><Icon name="fit" size={17}/></button></div></div>
        <Waveform peaks={song.peaks} duration={song.duration} {sections} {selection} position={transport.position} {view} onseek={seconds => action({ type: 'seek', seconds })} onselection={chooseLoop} onsection={selectSection} onview={next => { view = next; }}/>
        <div class="selection-bar">
          {#if selection}
            <div class="loop-times"><span class="endpoint">A</span><input aria-label="Loop start seconds" type="number" min="0" max={selection.end - .1} step="any" value={selection.start.toFixed(2)} onchange={e => { const r = range(Number(e.currentTarget.value), selection.end, song!.duration); if (r) void chooseLoop(r); }}/><span class="endpoint">B</span><input aria-label="Loop end seconds" type="number" min={selection.start + .1} max={song.duration} step="any" value={selection.end.toFixed(2)} onchange={e => { const r = range(selection.start, Number(e.currentTarget.value), song!.duration); if (r) void chooseLoop(r); }}/><span class="unit">seconds</span><span class="loop-length">{(selection.end - selection.start).toFixed(2)}s passage</span></div>
            <div class="selection-actions"><button class="text-button" onclick={fitLoop}>Fit passage</button><button class="text-button" onclick={() => startSection()}>Save as section</button><button class="icon-button" aria-label="Clear loop selection" title="X" onclick={clearLoop}><Icon name="close" size={16}/></button></div>
          {:else}<span class="selection-hint">Choose any passage to loop, or save it as a named section.</span><span class="selection-shortcut"><span class="keycap">A</span><span class="keycap">B</span>Set boundaries at the playhead</span>{/if}
        </div>

        <div class="transport-row">
          <div class="transport-buttons"><button class="icon-button" aria-label="Previous section" disabled={!sections.length} onclick={() => nextSection(-1)}><Icon name="back"/></button><button class="play-button" aria-label={transport.playing ? 'Pause' : 'Play'} title="Space" disabled={!native || !!transport.audio_error || !!busy} onclick={() => action({ type: 'toggle_play' })}><Icon name={transport.playing ? 'pause' : 'play'} size={27}/></button><button class="icon-button" aria-label="Next section" disabled={!sections.length} onclick={() => nextSection(1)}><Icon name="next"/></button></div>
          <div class="transport-time"><strong>{time(transport.position)}</strong><span>/ {time(song.duration)}</span><small>{transport.playing ? 'Playing' : transport.finished ? 'End of song' : 'Paused'}</small></div>
          <button class:active={project.loop_enabled} class="loop-toggle" onclick={toggleLoop} aria-pressed={project.loop_enabled} title="L"><Icon name="loop"/>{project.loop_enabled ? 'Loop on' : 'Loop off'}</button>
        </div>

        <div class="practice-controls">
          <section class="practice-control" aria-label="Playback speed"><div class="control-heading"><h3>Speed</h3><button class="icon-button" aria-label="Reset speed" title="Reset to original speed" onclick={() => controls(1, pitch)}><Icon name="reset" size={17}/></button></div><div class="control-number"><input aria-label="Speed percent" type="number" min="25" max="200" step="1" value={Math.round(speed * 100)} onchange={e => controls(Number(e.currentTarget.value) / 100, pitch)}/><span>%</span><small>of original tempo</small></div><input class="control-slider speed-slider" aria-label="Playback speed" type="range" min="25" max="200" step="1" value={speed * 100} oninput={e => controls(Number(e.currentTarget.value) / 100, pitch)}/><div class="control-bottom"><span>25%</span><div class="speed-presets">{#each [.5, .75, 1] as preset}<button class:chosen={Math.abs(speed - preset) < .005} onclick={() => controls(preset, pitch)}>{preset * 100}%</button>{/each}</div><span>200%</span></div></section>
          <section class="practice-control" aria-label="Playback pitch"><div class="control-heading"><h3>Pitch</h3><button class="icon-button" aria-label="Reset pitch" title="Reset to original pitch" onclick={() => controls(speed, 0)}><Icon name="reset" size={17}/></button></div><div class="control-number"><input aria-label="Pitch semitones" type="number" min="-12" max="12" step="0.01" value={pitch} onchange={e => controls(speed, Number(e.currentTarget.value))}/><span>st</span><small>{pitch === 0 ? 'Original pitch' : pitch > 0 ? 'Above original pitch' : 'Below original pitch'}</small></div><input class="control-slider pitch-slider" aria-label="Playback pitch" type="range" min="-12" max="12" step="0.1" value={pitch} oninput={e => controls(speed, Number(e.currentTarget.value))}/><div class="control-bottom"><span>−12 st</span><button class="pitch-home" onclick={() => controls(speed, 0)}>Original</button><span>+12 st</span></div></section>
        </div>
      {:else}
        <div class="empty-workbench"><svg viewBox="0 0 200 90" aria-hidden="true"><path d="M10 45h12l5-11 8 25 8-40 9 56 8-48 8 35 8-21h12m17 4h12l5-11 8 25 8-40 9 56 8-48 8 35 8-21h12" fill="none" stroke="#9caed1" stroke-width="3" stroke-linecap="round"/></svg><h2>A good place to get it right.</h2><p>Open a song, find your passage, and play it at your pace.</p><div><button class="button primary" onclick={() => open()} disabled={!!busy}><Icon name="folder"/>Open a song</button><button class="text-button" onclick={() => open(true)} disabled={!!busy}>Try the demo</button></div><span>WAV, MP3, FLAC, M4A and Ogg</span></div>
      {/if}
    </section>
  </main>
  <footer class="statusbar"><span>{#if undo.length && !busy}Section removed.<button class="text-button" onclick={undoDelete}>Undo</button>{:else}{busy || (song ? (saved ? 'Practice settings saved' : 'Saving practice settings…') : 'Ready when you are')}{#if song && saved}<Icon name="check" size={13}/>{/if}{/if}</span><span>{song ? (transport.device ?? (native ? 'No audio output' : 'Browser preview')) : 'Local songs • Independent speed and pitch'}</span><button onclick={() => helpDialog.showModal()}>Shortcuts <span class="keycap">Space</span>Play / pause</button></footer>
</div>

<dialog class="section-dialog" bind:this={sectionDialog} onclose={() => { dialogError = ''; }}>
  <form onsubmit={saveSection}><div class="dialog-heading"><h2>{sectionId ? 'Edit section' : 'Save a section'}</h2><button type="button" class="icon-button" aria-label="Close section editor" onclick={() => sectionDialog.close()}><Icon name="close"/></button></div><p>Keep a passage you’ll want to come back to.</p><label>Section name<input bind:this={nameInput} bind:value={sectionName} maxlength="128" placeholder="Verse, solo, a tricky phrase…" required/></label><div class="dialog-times"><label>Start (seconds)<input type="number" min="0" max={song?.duration ?? 0} step="any" bind:value={sectionStart} required/></label><label>End (seconds)<input type="number" min="0" max={song?.duration ?? 0} step="any" bind:value={sectionEnd} required/></label></div>{#if dialogError}<p class="form-error" role="alert">{dialogError}</p>{/if}<div class="dialog-actions"><button type="button" class="button" onclick={() => sectionDialog.close()}>Cancel</button><button class="button primary" type="submit">Save section</button></div></form>
</dialog>
<dialog class="help-dialog" bind:this={helpDialog}><div class="dialog-heading"><h2>Keep your hands on the guitar</h2><button class="icon-button" aria-label="Close shortcuts" onclick={() => helpDialog.close()}><Icon name="close"/></button></div><p>Shortcuts work when you’re not editing a field.</p><dl>{#each [['Space','Play / pause'],['L','Loop on / off'],['A / B','Set loop start / end'],['X','Clear A–B range'],['← / →','Seek 5 seconds'],['Shift + ← / →','Seek 0.1 seconds'],['[ / ]','Previous / next section'],['N','Add a section'],[native && navigator.platform.includes('Mac') ? '⌘ O' : 'Ctrl + O','Open a song'],['Esc','Turn looping off']] as shortcut}<div><dt>{shortcut[0]}</dt><dd>{shortcut[1]}</dd></div>{/each}</dl></dialog>
