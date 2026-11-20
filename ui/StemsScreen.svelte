<script lang="ts">
 // The Stems view, concept SC "guided steps" (docs/design/stems.md): a step
 // list (set up, separate, review) beside a pane holding the current step.
 // Attaching, replacing and removing are quiet links under the steps; tool
 // repair and removal live in Settings → Stem separation. What the pane shows
 // comes from stemsView() in stems-screen.ts.
 import Icon from './Icon.svelte';
 import StemProcessingProgress from './StemProcessingProgress.svelte';
 import StemsReview from './StemsReview.svelte';
 import StemsConfirm,{type StemsConfirmation} from './StemsConfirm.svelte';
 import {SvelteSet} from 'svelte/reactivity';
 import {tick} from 'svelte';
 import type {ProcessingEstimate,SeparationState} from './separation';
 import type {StemsState} from './stems-session';
 import {stemsView,type Attachment,type Work} from './stems-screen';
 import type {Keymap} from './keymap';
 import type {ButtonId} from './actions';
 import type {ActionPayload} from './action-dispatcher';
 let {active,reference,state:stemState,separation,attachment=null,space=null,keymap,available,run}:{active:boolean;reference:string;state:StemsState;separation:SeparationState;attachment?:Attachment|null;space?:ProcessingEstimate|null;keymap:Keymap;available:(id:ButtonId)=>boolean;run:(id:ButtonId,payload?:ActionPayload)=>void}=$props();
 const dismissed=new SvelteSet<string>();
 const view=$derived(stemsView({status:separation.status,reference,snapshot:stemState.snapshot,attachment,space,dismissed}));
 const pane=$derived(view.pane);
 const commandError=$derived(separation.error);
 let confirmation=$state<StemsConfirmation|null>(null);
 $effect(()=>{if(!active)confirmation=null;});
 const working=$derived(pane.kind==='work');
 /** True while a confirmation is open: it owns the keyboard and the menu bar. */
 export function confirming(){return confirmation!==null;}
 // Replacing a set always asks first; the old set stays until the new one is saved.
 // After a confirmation or a dismissal the pane changes under the pointer; focus
 // follows to the new pane's Cancel or heading instead of falling to the page.
 const paneKey=$derived(`${pane.kind}:${pane.kind==='work'||pane.kind==='stopped'?pane.work.id:''}`);
 let focusAfter:string|null=null;
 let content:HTMLElement|undefined=$state();
 function focusNextPane(){focusAfter=paneKey;}
 $effect(()=>{
  const key=paneKey;
  if(focusAfter===null||key===focusAfter)return;
  focusAfter=null;
  void tick().then(()=>{(content?.querySelector<HTMLElement>('[data-pane-focus]:not(:disabled)')??content?.querySelector<HTMLElement>('[data-pane-heading]'))?.focus({preventScroll:true});});
 });
 $effect(()=>{if(!active)focusAfter=null;});
 function dismiss(id:string){focusNextPane();dismissed.add(id);}
 const lowSpaceReason=$derived(view.lowSpace?`Needs about ${view.lowSpace.needed} of free disk space; ${view.lowSpace.free} is free`:null);
 function separate(){if(view.hasStems)confirmation='separate';else run('separation_start');}
 function attach(){if(view.hasStems)confirmation='attach';else run('attach_stems');}
 function retry(work:Work){if(work.kind==='setup')run('separation_install');else if(work.kind==='separation')separate();else attach();}
 const RETRY={setup:'Retry setup',separation:'Retry separation',attach:'Choose stems again…'};
 const settingsLink=()=>run('settings',{settingsSection:'stems'});
</script>
{#if active}
<main class="stems-screen" aria-label="Stems" aria-busy={working}>
 <aside class="rail" aria-label="Steps">
  <section class="card">
   <div class="intro">
    <h2>Stems for this song</h2>
    <span class="hint">Split the recording into instruments, review them, then mix them in Practice.</span>
   </div>
   <ol aria-label="Steps">
    {#each view.steps as step,index (step.key)}
     <li class={step.state} aria-current={step.state==='current'?'step':undefined}>
      <span class="badge" aria-hidden="true">{#if step.state==='done'}<Icon name="check" size={14}/>{:else}{index+1}{/if}</span>
      <div class="step-text"><strong>{step.title}</strong><span class="hint">{step.summary}</span></div>
     </li>
    {/each}
   </ol>
  </section>
  <div class="links" class:stacked={view.hasStems}>
   {#if view.hasStems}
    <button class="compact-button ghost" disabled={working||!available('attach_stems')} title={keymap.title('attach_stems')} onclick={()=>{confirmation='attach';}}><Icon name="folder" size={15}/>Replace with attached stems…</button>
    <button class="compact-button ghost danger" disabled={working||!available('remove_stems')} title={keymap.title('remove_stems')} onclick={()=>{confirmation='remove';}}>Remove stems…</button>
   {:else if pane.kind!=='unsupported'}
    <span class="hint">Made stems with another tool?</span>
    <button class="compact-button ghost" disabled={working||!available('attach_stems')} title={keymap.title('attach_stems')} onclick={attach}><Icon name="folder" size={15}/>Attach stems…</button>
   {/if}
  </div>
  <p class="hint footer">The original recording stays the source for the waveform and the measure map.</p>
 </aside>
 <section class="pane" aria-label={view.steps.find(step=>step.state==='current')?.title}>
  <div class="content" bind:this={content}>
   {#if commandError}<div class="command-error" role="alert"><Icon name="warning" size={16}/><span>{commandError}</span></div>{/if}
   {#if pane.kind==='checking'}
    <p class="hint" role="status">Checking stem separation…</p>
   {:else if pane.kind==='unsupported'}
    <h2 tabindex="-1" data-pane-heading>Separation can’t run on this computer</h2>
    <p class="lead">{pane.message} Stems made with another tool can still be attached.</p>
    <div><button class="compact-button primary" disabled={!available('attach_stems')} title={keymap.title('attach_stems')} onclick={attach}><Icon name="folder" size={15}/>Attach stems…</button></div>
   {:else if pane.kind==='setup'}
    <h2 tabindex="-1" data-pane-heading>Set up stem separation</h2>
    <p class="lead">A one-time download of the separation tools and the Roformer-SW model. Afterwards, separating works offline, for every song.</p>
    <dl><dt>Download</dt><dd class="number">{pane.download}, plus the separation tools</dd><dt>Meanwhile</dt><dd>Playback keeps working</dd></dl>
    <div><button class="compact-button primary" disabled={!available('separation_install')} title={keymap.title('separation_install')} onclick={()=>run('separation_install')}>Set up separation</button></div>
   {:else if pane.kind==='repair'}
    <h2 tabindex="-1" data-pane-heading>Stem separation needs repair</h2>
    <p class="lead">{pane.message} Reinstall the separation tools in Settings; stems the song already has stay.</p>
    <div><button class="compact-button primary" disabled={!available('settings')} title={keymap.title('settings')} onclick={settingsLink}>Repair in Settings</button></div>
   {:else if pane.kind==='ready'}
    <h2 tabindex="-1" data-pane-heading>Split this song into stems</h2>
    <p class="lead">Bass, drums, guitar, piano, vocals and other instruments. Near-silent instruments are left out.</p>
    <dl><dt>Takes</dt><dd>A few minutes; you can leave this screen</dd>
     {#if view.needed}<dt>Disk</dt><dd class="number">About {view.needed} free space, checked before starting</dd>{/if}
     <dt>Memory</dt><dd>Several GB while it runs</dd><dt>Playback</dt><dd>Paused until it finishes. Notes and measure map stay editable.</dd></dl>
    {@render waitingNote(pane.waiting)}{@render spaceNote()}
    <div><button class="compact-button primary" disabled={!!view.lowSpace||!available('separation_start')} title={lowSpaceReason??keymap.title('separation_start')} onclick={separate}>Separate song</button></div>
   {:else if pane.kind==='work'}
    {@const work=pane.work}
    <StemProcessingProgress {work}>{#snippet actions()}{#if work.cancel}<button class="compact-button" data-pane-focus disabled={!work.cancel.possible||!available('separation_cancel')} title={keymap.title('separation_cancel')} onclick={()=>run('separation_cancel')}>{work.cancel.label}</button>{/if}{/snippet}</StemProcessingProgress>
    <p class="hint">{work.kind==='setup'?'You can keep practising while it downloads.':work.kind==='separation'?'Playback is paused until separation finishes. Leave this screen if you like: progress stays visible on the other views.':'Playback is paused until the stems are saved.'}</p>
   {:else if pane.kind==='stopped'}
    {@const work=pane.work}
    <StemProcessingProgress {work}/>
    <div class="row">
     <button class="compact-button primary" disabled={!available(work.kind==='setup'?'separation_install':work.kind==='separation'?'separation_start':'attach_stems')} onclick={()=>retry(work)}>{RETRY[work.kind]}</button>
     {#if view.hasStems}<button class="compact-button" onclick={()=>dismiss(work.id)}>Back to current stems</button>
     {:else if work.kind==='attach'}<button class="compact-button" onclick={()=>dismiss(work.id)}>Back</button>{/if}
     {#if work.kind==='setup'}<button class="compact-button ghost" disabled={!available('settings')} title={keymap.title('settings')} onclick={settingsLink}>Separation settings</button>{/if}
    </div>
    <p class="hint">{work.kind==='setup'?'Files already verified are kept; the rest downloads again.':view.hasStems?'Your notes, measure map and current stems are unchanged.':'Your notes and measure map are unchanged.'}</p>
   {:else if pane.kind==='review'&&stemState.snapshot}
    <StemsReview snapshot={stemState.snapshot} banner={pane.banner} ondismiss={dismiss}>
     {#snippet actions()}
      {@render waitingNote(pane.waiting)}{#if view.toolsReady}{@render spaceNote()}{/if}
      <div class="row">
       {#if view.toolsReady}<button class="compact-button" disabled={!!view.lowSpace||!available('separation_start')} title={lowSpaceReason??keymap.title('separation_start')} onclick={separate}>Separate again…</button>
       {:else if view.repair}<button class="compact-button" disabled={!available('settings')} title={keymap.title('settings')} onclick={settingsLink}>Repair in Settings</button>
       {:else if separation.status&&separation.status.runtime.supported!==false}<button class="compact-button" disabled={!available('separation_install')} title={keymap.title('separation_install')} onclick={()=>run('separation_install')}>Set up separation</button>{/if}
      </div>
     {/snippet}
    </StemsReview>
   {/if}
  </div>
 </section>
</main>
{/if}
<StemsConfirm kind={active?confirmation:null} {keymap} {available} oncancel={()=>{confirmation=null;}} onconfirm={id=>{confirmation=null;focusNextPane();run(id);}}/>
{#snippet spaceNote()}
 {#if view.lowSpace}<div class="waiting" role="status"><Icon name="warning" size={16}/><span><b>Not enough free disk space.</b> Separating needs about {view.lowSpace.needed}; {view.lowSpace.free} is free. Free some space, then come back to Stems.</span></div>{/if}
{/snippet}
{#snippet waitingNote(waiting:string|null)}
 {#if waiting}<div class="waiting" role="status"><Icon name="hourglass" size={16}/><span>{waiting} is running. Separation can start when it finishes.</span></div>{/if}
{/snippet}
<style>
 .stems-screen{flex:1;min-height:0;display:flex;gap:18px}
 .rail{width:300px;flex:none;display:flex;flex-direction:column;gap:12px;min-height:0}
 .card{border:1px solid var(--border);border-radius:10px;background:var(--sidebar);overflow:hidden}
 .intro{padding:12px 16px 16px;display:flex;flex-direction:column;gap:6px}.intro h2{font-size:18px}
 ol{list-style:none;margin:0;padding:0}
 li{display:flex;gap:12px;padding:16px;border-top:1px solid var(--border)}
 li.current{background:var(--blue-wash)}
 .badge{flex:none;width:26px;height:26px;border-radius:8px;display:flex;align-items:center;justify-content:center;font-size:12px;font-weight:700;background:var(--border);color:var(--text-secondary)}
 .current .badge{background:var(--blue);color:var(--on-accent)}.done .badge{background:var(--success-bg);color:var(--success-text)}
 .step-text{flex:1;min-width:0;display:flex;flex-direction:column;gap:3px}.step-text strong{font-size:13px}
 .todo .step-text{opacity:.55}
 .links{display:flex;align-items:center;gap:4px;flex-wrap:wrap}.links.stacked{flex-direction:column;align-items:flex-start}.links .compact-button{padding:0 6px}.links .danger{color:var(--error-text)}
 .rail p.footer{margin-top:auto}
 .hint{font-size:12px;line-height:1.45;color:var(--text-secondary)}p.hint{margin:0}
 .pane{flex:1;min-width:0;padding:24px 28px;border:1px solid var(--border);border-radius:10px;background:var(--surface);overflow:auto;overscroll-behavior:contain}
 .content{max-width:860px;display:flex;flex-direction:column;gap:14px}
 .content>h2{font-size:20px}
 .lead{margin:0;font-size:13px;line-height:1.5;color:var(--text-secondary);max-width:640px}
 dl{margin:0;display:grid;grid-template-columns:auto 1fr;gap:6px 18px;font-size:12px}dt{color:var(--text-secondary)}dd{margin:0}.number{font-variant-numeric:tabular-nums}
 .row{display:flex;align-items:center;gap:8px;flex-wrap:wrap}
 .waiting,.command-error{display:flex;gap:10px;align-items:center;padding:9px 12px;border-radius:8px;font-size:12px}
 .waiting{border:1px solid var(--amber-border);background:var(--amber-bg);color:var(--amber-text)}
 .command-error{border:1px solid var(--error-border);background:var(--error-bg);color:var(--error-text);overflow-wrap:anywhere}
</style>
