<script lang="ts">
 // Setup, separation or attachment work on the Stems screen: one overall bar
 // with a fixed title, the phase and elapsed time beneath it, and the step
 // checklist. While work runs the checklist sits behind "Show steps"; when it
 // stops, the list is open with the error and its details to copy (stems.md
 // decision 4). Unmeasured work stays indeterminate; no ETA.
 import {tick,untrack,type Snippet} from 'svelte';
 import Icon from './Icon.svelte';
 import {ChecklistFollow} from './checklist-follow';
 import {percent,type Work} from './stems-screen';
 import type {StemProcessingStep} from './stems';
 let {work,actions}:{work:Work;actions?:Snippet}=$props();
 const workflow=$derived(work.progress?.workflow);
 const value=$derived(percent(work.progress));
 let stepsShown=$state(false);
 const stepsOpen=$derived(work.stopped||stepsShown);
 let copied=$state(false);
 const activeId=$derived(workflow?.steps.find(step=>step.state==='active')?.id);
 let list:HTMLUListElement|undefined=$state();
 const follow=new ChecklistFollow();
 let previousAttempt:string|undefined;
 let wasRunning=false;
 // Keys the scrollable list handles itself; every other key reaches the shortcuts.
 const LIST_KEYS='ArrowUp ArrowDown PageUp PageDown Home End';
 function geometry(){
  const row=list?.querySelector<HTMLElement>('[aria-current="step"]');
  if(!list||!row)return null;
  const viewport=list.getBoundingClientRect(), item=row.getBoundingClientRect();
  return {top:viewport.top+list.clientTop,bottom:viewport.top+list.clientTop+list.clientHeight,rowTop:item.top,rowBottom:item.bottom};
 }
 function scrolled(){const value=geometry();if(value)follow.scrolled(value);}
 // The list follows the active step unless the player scrolls away from it.
 $effect(()=>{
  const active=activeId,attempt=work.id,running=work.running,open=stepsOpen;
  untrack(()=>{
   if(attempt!==previousAttempt||(running&&!wasRunning))follow.reset();
   previousAttempt=attempt;wasRunning=running;
  });
  if(!running||!active||!open)return;
  let disposed=false;
  tick().then(()=>{if(disposed)return;const value=geometry();if(value&&list)list.scrollTop+=follow.reveal(value);});
  return ()=>{disposed=true;};
 });
 function stepFraction(step:StemProcessingStep){return (step.state==='complete'||step.state==='skipped')?1:step.total&&step.total>0?Math.max(0,Math.min(1,step.completed/step.total)):0;}
 function stepStatus(step:StemProcessingStep){
  if(step.state==='skipped')return 'Skipped';
  if(step.state==='complete')return 'Complete';
  if(step.state==='active')return work.stopped?work.cancelled?'Cancelled':'Stopped':step.total?`${Math.floor(stepFraction(step)*100)} %`:'Working';
  return 'Pending';
 }
 async function copy(){try{await navigator.clipboard.writeText(work.details);copied=true;setTimeout(()=>{copied=false;},1500);}catch{copied=false;}}
</script>
<section class="work" class:stopped={work.stopped} class:failed={work.stopped&&!work.cancelled} aria-label={work.title}>
 <div class="heading"><h2 tabindex="-1" data-pane-heading>{work.title}</h2><span>{work.stopped||value===null?'':`${value} %`}</span></div>
 <progress aria-label={work.title} max="100" value={work.stopped?(value??0):value??undefined}></progress>
 <div class="status-row"><span class="phase" role="status">{work.phase}</span>{#if work.elapsed}<span class="elapsed">{work.elapsed}</span>{/if}{@render actions?.()}</div>
 {#if work.stopped&&!work.cancelled&&(work.error||work.details)}
  <div class="error" role="alert"><Icon name="warning" size={16}/><span class="copyable"><b>{work.error||'The work stopped unexpectedly.'}</b></span>
   {#if work.details}<button class="compact-button ghost" onclick={copy}><Icon name="copy" size={14}/>{copied?'Copied':'Copy details'}</button>{/if}</div>
 {/if}
 {#if workflow?.steps.length}
  {#if !work.stopped}<button class="compact-button ghost toggle" aria-expanded={stepsShown} aria-controls="stem-steps" onclick={()=>{stepsShown=!stepsShown;}}>{stepsShown?'Hide steps':'Show steps'}<span class="chevron" class:open={stepsShown} aria-hidden="true">▾</span></button>{/if}
  {#if stepsOpen}
   <!-- svelte-ignore a11y_no_noninteractive_tabindex (Keyboard focus makes this scrollable list reachable.) -->
   <ul id="stem-steps" bind:this={list} onscroll={scrolled} class="steps" aria-label="Processing steps" tabindex="0" data-owns-keys={LIST_KEYS}>
    {#each workflow.steps as step (step.id)}
     <li class:active={step.state==='active'} class:complete={step.state==='complete'||step.state==='skipped'} aria-current={step.state==='active'?'step':undefined}>
      <span class="circle" class:running={work.running&&step.state==='active'&&step.total===null} style:--sweep={`${stepFraction(step)*360}deg`} aria-hidden="true"></span>
      <span class="label">{step.label}</span><span class="step-status">{stepStatus(step)}</span>
     </li>
    {/each}
   </ul>
  {/if}
 {/if}
</section>
<style>
 .work{display:flex;flex-direction:column;gap:8px}
 .heading{display:flex;align-items:baseline;gap:12px}.heading h2{flex:1;font-size:17px;font-weight:600}.heading span{font-size:13px;color:var(--text-secondary)}
 progress{display:block;width:100%;height:10px;border:0;border-radius:5px;overflow:hidden;background:var(--progress-track);accent-color:var(--blue)}
 progress::-webkit-progress-bar{background:var(--progress-track);border-radius:5px}progress::-webkit-progress-value{background:var(--blue);border-radius:5px}progress::-moz-progress-bar{background:var(--blue);border-radius:5px}
 .stopped progress::-webkit-progress-value{background:var(--border-strong)}.stopped progress::-moz-progress-bar{background:var(--border-strong)}.failed progress::-webkit-progress-value{background:var(--error-text)}.failed progress::-moz-progress-bar{background:var(--error-text)}
 .status-row{display:flex;align-items:center;gap:12px;min-height:30px;font-size:12px;color:var(--text-secondary)}.phase{flex:1}.failed .phase{color:var(--error-text)}
 .error{display:flex;gap:10px;align-items:flex-start;padding:10px 12px;border:1px solid var(--error-border);border-radius:8px;background:var(--error-bg);color:var(--error-text);font-size:12px;line-height:1.45}
 .error>span{flex:1;overflow-wrap:anywhere}.error .compact-button{height:22px;padding:0 4px;color:var(--error-text)}
 .toggle{align-self:flex-start;height:22px;padding:0 4px;margin-left:-4px}.chevron{display:inline-block;font-size:10px}.chevron.open{transform:rotate(180deg)}
 .steps{list-style:none;margin:0;padding:4px 12px 4px 0;max-height:210px;overflow-y:auto;overscroll-behavior:contain;border-block:1px solid var(--border)}
 .steps li{display:flex;align-items:center;gap:10px;min-height:30px;font-size:12px;color:var(--text-secondary)}
 .steps .active{color:var(--text);font-weight:600}.steps .complete{color:var(--text)}.failed .steps .active{color:var(--error-text)}
 .circle{width:13px;height:13px;flex:none;border:1px solid var(--border-strong);border-radius:50%;background:conic-gradient(var(--success-text) var(--sweep),transparent var(--sweep))}
 .active .circle{border-color:var(--success-text)}.complete .circle{border-color:var(--success-text);background:var(--success-text)}.stopped .active .circle{border-color:var(--border-strong);background:transparent}.failed .active .circle{border-color:var(--error-text)}
 .label{flex:1}.step-status{font-size:11px;font-weight:400;color:var(--text-secondary)}
 .running{animation:processing-pulse 1.2s ease-in-out infinite}@keyframes processing-pulse{50%{opacity:.4}}@media(prefers-reduced-motion:reduce){.running{animation:none}}
</style>
