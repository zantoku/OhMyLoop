<script lang="ts" module>
 export type StemsConfirmation='separate'|'attach'|'remove';
</script>
<script lang="ts">
 // The Stems screen's confirmations: replacing the set by separating again or
 // by attaching another, and removing it. A modal dialog owns every key; Esc
 // and the leading X cancel, Enter runs the focused default. Removal defaults
 // to Cancel, so Enter never deletes by accident.
 import Icon from './Icon.svelte';
 import type {ButtonId} from './actions';
 import type {Keymap} from './keymap';
 let {kind,keymap,available,onconfirm,oncancel}:{kind:StemsConfirmation|null;keymap:Keymap;available:(id:ButtonId)=>boolean;onconfirm:(id:ButtonId)=>void;oncancel:()=>void}=$props();
 const COPY:Record<StemsConfirmation,{title:string;text:string;confirm:string;action:ButtonId}>={
  separate:{title:'Replace stems?',text:'The current stems stay until the new set is checked and saved. Mix settings for matching instruments are kept.',confirm:'Separate and replace',action:'separation_start'},
  attach:{title:'Replace stems?',text:'Choose a folder of stems made with another tool. The current stems stay until the new set is checked and saved. Mix settings for matching instruments are kept.',confirm:'Choose stems…',action:'attach_stems'},
  remove:{title:'Remove stems?',text:'The stems and their playback data are deleted. The recording, measure map, sections and notes stay.',confirm:'Remove stems',action:'remove_stems'}
 };
 let dialog:HTMLDialogElement;
 const copy=$derived(kind?COPY[kind]:null);
 $effect(()=>{
  if(kind&&!dialog.open){dialog.showModal();dialog.querySelector<HTMLElement>('[data-default]')?.focus();}
  else if(!kind&&dialog.open)dialog.close();
 });
</script>
<dialog bind:this={dialog} class="stems-confirm" aria-label={copy?.title} oncancel={event=>{event.preventDefault();oncancel();}}>
 {#if kind&&copy}
  <div class="heading"><button class="icon-button" aria-label="Close" onclick={oncancel}><Icon name="close" size={16}/></button><h2>{copy.title}</h2></div>
  <p>{copy.text}</p>
  <div class="actions">
   <button class="compact-button" data-default={kind==='remove'?'':undefined} onclick={oncancel}>Cancel</button>
   <button class="compact-button" class:primary={kind!=='remove'} class:danger-fill={kind==='remove'} data-default={kind==='remove'?undefined:''} disabled={!available(copy.action)} title={keymap.title(copy.action)} onclick={()=>onconfirm(copy.action)}>{copy.confirm}</button>
  </div>
 {/if}
</dialog>
<style>
 dialog[open]{width:420px;display:flex;flex-direction:column;gap:14px;padding:16px 20px 18px;border-radius:12px}
 .heading{display:flex;align-items:center;gap:8px;margin-left:-8px}.heading h2{margin:0;font-size:16px}
 p{margin:0;font-size:13px;line-height:1.5;color:var(--text-secondary)}
 .actions{display:flex;justify-content:flex-end;gap:8px}
 .danger-fill{border-color:var(--error-text);background:var(--error-text);color:var(--on-accent)}
 .danger-fill:hover:not(:disabled){filter:brightness(1.08);background:var(--error-text)}
</style>
