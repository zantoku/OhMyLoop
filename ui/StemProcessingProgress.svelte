<script lang="ts">
 import {tick,untrack} from 'svelte';
 import {ChecklistFollow} from './checklist-follow';
 import type {StemProgress,StemProcessingStep} from './stems';
 let {progress,busy,failed=false,operation='Processing stems',title,completionTitle='Stems ready'}:{progress:StemProgress|null;busy:boolean;failed?:boolean;operation?:string;title?:string;completionTitle?:string}=$props();
 const workflow=$derived(progress?.workflow);
 const known=$derived(!!workflow&&workflow.total>0);
 const fraction=$derived(known?Math.max(0,Math.min(1,workflow!.completed/workflow!.total)):0);
 const finished=$derived(known&&fraction===1&&!failed);
 const phaseTitle=$derived(failed?'Processing stopped':finished?completionTitle:operation.replace(/…$/,''));
 const current=$derived(workflow?.steps.find(step=>step.state==='active'));
 const activeId=$derived(current?.id);
 let list:HTMLUListElement|undefined=$state();
 const follow=new ChecklistFollow();
 let previousAttempt:string|undefined;
 let wasBusy=false;
 function geometry(){
  const row=list?.querySelector<HTMLElement>('[aria-current="step"]');
  if(!list||!row)return null;
  const viewport=list.getBoundingClientRect(), item=row.getBoundingClientRect();
  return {top:viewport.top+list.clientTop,bottom:viewport.top+list.clientTop+list.clientHeight,rowTop:item.top,rowBottom:item.bottom};
 }
 function scrolled(){const value=geometry();if(value)follow.scrolled(value);}
 $effect(()=>{
  const active=activeId,attempt=progress?.operation_id??progress?.reference_token,running=busy;
  untrack(()=>{
   if(attempt!==previousAttempt||(running&&!wasBusy))follow.reset();
   previousAttempt=attempt;wasBusy=running;
  });
  if(!running||!active)return;
  let disposed=false;
  tick().then(()=>{if(disposed)return;const value=geometry();if(value&&list)list.scrollTop+=follow.reveal(value);});
  return ()=>{disposed=true;};
 });
 function stepFraction(step:StemProcessingStep){return (step.state==='complete'||step.state==='skipped')?1:step.total&&step.total>0?Math.max(0,Math.min(1,step.completed/step.total)):0;}
</script>
<section class="processing-progress" aria-label="Stem processing progress">
 <div class="progress-heading"><strong>{title??phaseTitle}</strong><span>{known?`${Math.floor(fraction*100)}%`:''}</span></div>
 <progress aria-label="Overall stem processing" max="1" value={known?fraction:undefined}></progress>
 <div class="processing-status" role="status">{title!==undefined?phaseTitle:failed?'Some steps could not complete.':finished?'All processing steps completed.':current?.label??(busy?'Getting ready…':'Processing paused.')}</div>
 {#if workflow?.steps.length}
 <!-- svelte-ignore a11y_no_noninteractive_tabindex (Keyboard focus makes this scrollable list reachable.) -->
 <ul bind:this={list} onscroll={scrolled} class="processing-steps" aria-label="Processing steps" tabindex="0">
  {#each workflow.steps as step (step.id)}
   <li class:active={step.state==='active'} class:complete={step.state==='complete'||step.state==='skipped'} aria-current={step.state==='active'?'step':undefined}>
    <span class="step-circle" class:running={busy&&step.state==='active'&&step.total===null} style:--sweep={`${stepFraction(step)*360}deg`} aria-hidden="true"></span>
    <span class="step-label">{step.label}</span><span class="step-status">{step.state==='skipped'?'Skipped':step.state==='complete'?'Complete':step.state==='active'?(failed?'Stopped':step.total?`${Math.floor(stepFraction(step)*100)}%`:'Working'):'Pending'}</span>
   </li>
  {/each}
 </ul>
 {/if}
</section>
<style>
 .processing-progress{width:100%;flex-shrink:0;display:grid;gap:10px}.progress-heading{display:flex;justify-content:space-between;align-items:center;gap:12px;font-size:14px}.progress-heading strong{font-weight:600}.progress-heading>span{font-size:12px;color:var(--text-secondary);font-variant-numeric:tabular-nums}progress{display:block;width:100%;height:18px;border:0;border-radius:5px;overflow:hidden;background:var(--progress-track);accent-color:var(--blue)}progress::-webkit-progress-bar{background:var(--progress-track);border-radius:5px}progress::-webkit-progress-value{background:var(--blue);border-radius:5px}progress::-moz-progress-bar{background:var(--blue);border-radius:5px}.processing-status{font-size:12px;color:var(--muted);min-height:18px}.processing-steps{list-style:none;padding:4px 12px 4px 0;margin:0;max-height:min(34vh,300px);overflow-y:auto;overscroll-behavior:contain;border-block:1px solid var(--border);scrollbar-gutter:stable}.processing-steps li{display:flex;align-items:center;gap:12px;padding:9px 0;min-height:36px;font-size:12px;color:var(--muted)}.processing-steps .active{color:var(--text);font-weight:600}.processing-steps .complete{color:var(--text-secondary)}.step-circle{width:15px;height:15px;flex:0 0 15px;border:1px solid var(--border-strong);border-radius:50%;background:conic-gradient(var(--success-text) var(--sweep),transparent var(--sweep))}.active .step-circle{border-color:var(--success-text)}.complete .step-circle{border-color:var(--success-text);background:var(--success-text)}.step-label{flex:1}.step-status{font-size:10px;font-weight:400;color:var(--muted)}.running{animation:processing-pulse 1.2s ease-in-out infinite}@keyframes processing-pulse{50%{opacity:.4}}@media(prefers-reduced-motion:reduce){.running{animation:none}}
</style>
