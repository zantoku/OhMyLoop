<script lang="ts">
 // Step 3, the song's stem set (stems.md decision 7): the instruments, with
 // near-silent ones marked, and where the set came from. Playback state shows
 // only when it falls back to the original; headroom and the model sit behind
 // Details. Mixing happens in Practice.
 import type {Snippet} from 'svelte';
 import Icon from './Icon.svelte';
 import {modelName,roleName,skippedRoles,STEM_ROLES,type StemsSnapshot} from './stems';
 let {snapshot,banner,ondismiss,actions}:{snapshot:StemsSnapshot;banner:{id:string;text:string}|null;ondismiss:(id:string)=>void;actions:Snippet}=$props();
 const skipped=$derived(skippedRoles(snapshot));
 const chips=$derived(STEM_ROLES.filter(role=>snapshot.tracks.some(track=>track.role===role)||skipped.includes(role)).map(role=>({role,skipped:skipped.includes(role)})));
 const model=$derived(modelName(snapshot.model));
 const source=$derived(snapshot.separated?`Separated here${model?` with ${model}`:''}`:`Attached from another tool${model?` · made with ${model}`:''}`);
 const headroom=$derived(snapshot.attenuation===null?'Unavailable':`${(20*Math.log10(snapshot.attenuation)).toFixed(2).replace('-','−')} dB`);
</script>
{#if banner}
 <div class="banner" role="status"><Icon name="check" size={16}/><span>{banner.text}</span>
  <button class="icon-button" aria-label="Dismiss" onclick={()=>ondismiss(banner.id)}><Icon name="close" size={14}/></button></div>
{/if}
<div class="title"><h2 tabindex="-1" data-pane-heading>Review the stems</h2><span class="hint">{source}</span></div>
{#if !snapshot.available}
 <div class="note warning" role="status"><Icon name="warning" size={16}/><span><b>Playing the original recording.</b> {snapshot.warning??'The stems can’t be played.'} Your mix is kept for when the stems work again.</span></div>
{:else if snapshot.warning}
 <div class="note" role="status"><Icon name="warning" size={16}/><span>{snapshot.warning}</span></div>
{/if}
<ul class="chips" aria-label="Instruments">
 {#each chips as chip (chip.role)}
  <li class:skipped={chip.skipped} title={chip.skipped?`${roleName(chip.role)} was near-silent and left out`:roleName(chip.role)}><span class="name">{roleName(chip.role)}</span>{#if chip.skipped}<span class="note-text">near-silent</span>{/if}</li>
 {/each}
</ul>
<p class="hint">In Practice, mute, solo or turn down any instrument.</p>
<details><summary>Details</summary>
 <dl><dt>Model</dt><dd>{model??'Not recorded'}</dd><dt>Shared headroom</dt><dd class="number">{headroom}</dd><dt>Stored in</dt><dd>This song’s library folder</dd></dl>
</details>
{@render actions()}
<style>
 .banner{display:flex;gap:10px;align-items:center;padding:6px 6px 6px 12px;border:1px solid var(--success-border);border-radius:8px;background:var(--success-bg);color:var(--success-text);font-size:12px}
 .banner>span{flex:1}.banner .icon-button{width:24px;height:24px;color:var(--success-text)}
 .title{display:flex;align-items:baseline;gap:12px}.title h2{flex:1;font-size:20px}
 .hint{font-size:12px;line-height:1.45;color:var(--text-secondary)}p.hint{margin:0}
 .note{display:flex;gap:10px;align-items:flex-start;padding:9px 12px;border:1px solid var(--border-strong);border-radius:8px;font-size:12px;line-height:1.45;color:var(--text-secondary)}
 .note.warning{border-color:var(--amber-border);background:var(--amber-bg);color:var(--amber-text)}.note>span{flex:1;overflow-wrap:anywhere}
 .chips{list-style:none;margin:0;padding:0;display:flex;flex-wrap:wrap;gap:8px}
 .chips li{display:inline-flex;align-items:center;gap:6px;height:30px;padding:0 12px;border:1px solid var(--success-border);border-radius:15px;background:var(--success-bg);color:var(--text);font-size:12px;font-weight:600}
 .chips li.skipped{border-color:var(--border);background:transparent;color:var(--text-secondary)}.skipped .name{text-decoration:line-through}
 .note-text{font-size:11px;font-weight:400;color:var(--text-secondary)}
 details{font-size:12px;color:var(--text-secondary)}summary{width:max-content}
 dl{margin:8px 0 0;display:grid;grid-template-columns:auto 1fr;gap:6px 18px}dt{color:var(--text-secondary)}dd{margin:0;color:var(--text)}.number{font-variant-numeric:tabular-nums}
</style>
