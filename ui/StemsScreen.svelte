<script lang="ts">
 import Icon from './Icon.svelte';
 import {roleName,type StemProgress} from './stems';
 import type {StemsState} from './stems-session';
 import type {Keymap} from './keymap';
 import type {ButtonId} from './actions';
 let {active,state:stemState,progress,busy,keymap,available,run}:{active:boolean;state:StemsState;progress:StemProgress|null;busy:boolean;keymap:Keymap;available:(id:ButtonId)=>boolean;run:(id:ButtonId)=>void}=$props();
 let confirmation=$state<'replace'|'remove'|null>(null);
 $effect(()=>{if(!active)confirmation=null;});
</script>
{#if active}
<main class="stems-screen" aria-label="Stems" aria-busy={busy}>
 <div class="screen-title"><button class="icon-button screen-close" disabled={!available('close_stems')} aria-label="Close stems" title={keymap.title('close_stems')} onclick={()=>run('close_stems')}><Icon name="close"/></button><h2>Stems</h2></div>
 <p>Attach completed separation outputs to isolate instruments during practice. The reference recording remains the source for waveforms and measure mapping.</p>
 {#if stemState.error}<div class="notice error" role="alert">{stemState.error}</div>{/if}
 {#if stemState.snapshot?.warning}<div class="notice" role="status">{stemState.snapshot.warning}{#if !stemState.snapshot.available} Original playback is available; track controls are disabled.{/if}</div>{/if}
 {#if busy}<div role="status">{progress?.phase??'Preparing stems…'}{#if progress?.total} · {Math.round(progress.completed/progress.total*100)}%{/if}</div>{/if}
 {#if stemState.snapshot?.set_id}
 <dl><dt>Model</dt><dd>{stemState.snapshot.model??'Imported stem set'}</dd><dt>Tracks</dt><dd>{stemState.snapshot.tracks.map(track=>roleName(track.role)).join(', ')}</dd><dt>Shared headroom</dt><dd>{stemState.snapshot.attenuation===null?'Unavailable':`${(20*Math.log10(stemState.snapshot.attenuation)).toFixed(2)} dB`}</dd><dt>Playback</dt><dd>{stemState.snapshot.available?'Ready':'Original fallback'}</dd></dl>
 <div class="actions"><button class="button" disabled={!available('attach_stems')} title={keymap.title('attach_stems')} onclick={()=>confirmation='replace'}>Replace stems…</button><button class="button" disabled={!available('remove_stems')} title={keymap.title('remove_stems')} onclick={()=>confirmation='remove'}>Remove stems…</button></div>
 {:else}<p>No stems attached. Choose a completed output directory containing its manifest and declared stem audio.</p><button class="button" disabled={!available('attach_stems')} title={keymap.title('attach_stems')} onclick={()=>run('attach_stems')}>Attach stems…</button>{/if}
 {#if confirmation}
 <div class="confirmation" role="alertdialog" tabindex="-1" aria-label={confirmation==='remove'?'Remove stems confirmation':'Replace stems confirmation'}>
  <div class="screen-title"><button class="icon-button" disabled={busy} aria-label="Close confirmation" onclick={()=>confirmation=null}><Icon name="close"/></button><h3>{confirmation==='remove'?'Remove stems?':'Replace stems?'}</h3></div>
  <p>{confirmation==='remove'?'The owned stems and playback cache will be permanently deleted. The recording, map, sections and notes remain.':'The current stems are removed only after the new set is validated, copied and saved. Matching track gains and mutes are retained.'}</p>
  <button class="button" disabled={busy} onclick={()=>{const action=confirmation==='remove'?'remove_stems':'attach_stems';confirmation=null;run(action);}}>{confirmation==='remove'?'Remove stems':'Choose replacement…'}</button>
 </div>
 {/if}
</main>
{/if}
<style>
 .stems-screen{display:flex;flex-direction:column;align-items:flex-start;flex:1;min-height:0;overflow:auto;background:var(--surface);border:1px solid var(--border);border-radius:9px;padding:22px 25px;gap:20px}.stems-screen p{max-width:720px;line-height:1.6;margin:0;font-size:13px;color:var(--text-secondary)}h2{font-size:23px;font-weight:500}dl{display:grid;grid-template-columns:auto 1fr;gap:12px 24px;font-size:13px}dt{color:var(--muted)}dd{margin:0}.actions{display:flex;gap:12px;flex-wrap:wrap}.confirmation{padding:18px;border:1px solid var(--border-strong);border-radius:7px;display:grid;gap:16px;max-width:600px}.confirmation .button{justify-self:start}@media(max-width:700px){.stems-screen{padding:18px 17px}}
</style>
