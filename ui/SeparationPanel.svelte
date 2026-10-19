<script lang="ts">
 import Icon from './Icon.svelte';
 import StemProcessingProgress from './StemProcessingProgress.svelte';
 import type {SeparationState} from './separation';
 import type {StemProgress} from './stems';
 import type {ButtonId} from './actions';
 let {state:separationState,processingBytes=null,hasStems,reference,available,run}:{state:SeparationState;processingBytes?:number|null;hasStems:boolean;reference:string;available:(id:ButtonId)=>boolean;run:(id:ButtonId)=>void}=$props();
 let confirmation=$state<'replace'|'model'|'runtime'|null>(null);
 const status=$derived(separationState.status);
 const job=$derived(status?.job);
 const install=$derived(status?.install);
 const setupProgress=$derived<StemProgress|null>(install?.progress?{reference_token:'',operation_id:install.id,phase:install.phase,completed:install.progress.completed,total:install.progress.total,workflow:install.progress.workflow}:null);
 const ready=$derived(!!status?.runtime.ready&&status.model_ready);
 const supported=$derived(status?.runtime.supported!==false);
 const progress=$derived<StemProgress|null>(job?.progress?{reference_token:job.reference_token??'',operation_id:job.id,phase:job.phase,completed:job.progress.completed,total:job.progress.total,workflow:job.progress.workflow}:null);
 const latestInstall=$derived(!!install?.running||!!install?.id&&!job?.id||(install?.order??0)>(job?.order??0));
 const current=$derived(latestInstall?install:job?.running||job?.reference_token===reference&&job.phase!=='Idle'?job:install?.phase!=='Idle'?install:null);
 const elapsed=$derived(current?`${Math.floor(current.elapsed_seconds/60)}:${String(Math.floor(current.elapsed_seconds%60)).padStart(2,'0')}`:'');
 function separate(){if(hasStems)confirmation='replace';else run('separation_start');}
</script>
<section class="separation-panel" aria-label="Stem separation">
 {#if job?.reference_token===reference&&(job.running||job.id)&&(!latestInstall||job.running)}
 <StemProcessingProgress {progress} busy={job.running} failed={job.phase==='Failed'||job.phase==='Cancelled'} operation={job.phase} title="Splitting song into stems"/>
 <div class="job-controls"><span role="status">{job.phase} · {elapsed} elapsed</span>{#if job.running}<button class="button" disabled={!available('separation_cancel')} onclick={()=>run('separation_cancel')}>{job.phase==='Stopping'?'Stopping…':job.phase==='Finishing save'?'Finishing save…':'Cancel'}</button>{:else if job.phase==='Complete'}<button class="button primary" onclick={()=>run('separation_practice')}>Go to Practice</button>{:else}<button class="button" disabled={!available('separation_start')} onclick={separate}>Retry separation</button>{/if}</div>
 {/if}
  {#if latestInstall&&install?.id&&install.phase!=='Idle'}
   <StemProcessingProgress progress={setupProgress} busy={install.running} failed={install.phase==='Failed'||install.phase==='Cancelled'} operation={install.phase} title="Setting up stem separation" completionTitle="Setup complete"/>
   <div class="job-controls"><span>{install.phase} · {elapsed} elapsed</span>{#if install.running}<button class="button" disabled={!available('separation_cancel')} onclick={()=>run('separation_cancel')}>{install.phase==='Stopping'?'Stopping…':'Cancel'}</button>{/if}</div>
  {/if}
 {#if separationState.error||current?.error}<div class="notice error" role="alert">{separationState.error||current?.error}</div>{/if}
 {#if current?.details}<details class="diagnostics"><summary>Processing details</summary><pre>{current.details}</pre><button class="text-button" onclick={()=>navigator.clipboard.writeText(current!.details)}>Copy details</button></details>{/if}
 <div class="model-card">
  <div class="model-heading"><h3>Roformer-SW</h3><span class="model-state">{install?.running?install.phase:!status?'Checking availability…':!supported?'Unavailable on this device':ready?'Ready':'Setup required'}</span></div>
  <p>Separate bass, drums, guitar, piano, vocals and other instruments. Near-silent tracks are omitted.</p>
  <p class="small">Setup downloads private separation tools and a {(status?(status.package_bytes/1e6).toFixed(1):'699.4')} MB model. Installed separation works offline.</p>
  <dl class="component-status"><div><dt>Separation tools</dt><dd>{status?.runtime.ready?'Ready':!supported?'Unsupported':'Setup required'}</dd></div><div><dt>Model</dt><dd>{status?.model_ready?'Ready':'Download required'}</dd></div></dl>
  {#if status&&!status.runtime.ready}<p class="small" role="status">{status.runtime.message}</p>{/if}

  {#if !install?.running}
   {#if !ready}<button class="button primary" disabled={!supported||!available('separation_install')} onclick={()=>run('separation_install')}>{install?.phase==='Failed'||install?.phase==='Cancelled'?'Retry setup':'Set up stem separation'}</button>
   {:else}<button class="button primary" disabled={!available('separation_start')} onclick={separate}>Separate song</button>{/if}
  {/if}
  {#if status?.heavy_work&&!job?.running}<p class="small" role="status">{status.heavy_work} is running. Separation waits until it finishes.</p>{/if}
  {#if processingBytes!==null}<p class="small">Processing storage: approximately {(processingBytes/1e9).toFixed(2)} GB · checked before starting</p>{/if}
  <p class="small">Processing can use several GiB of memory. Playback pauses until separation finishes.</p>
  <details><summary>Separation management</summary>
   <p class="small">Tools and models are shared across songs and libraries. Removing them leaves recordings, completed stems, notes and maps intact.</p>
   <div class="management-controls"><button class="button" disabled={!available('separation_reinstall_runtime')} onclick={()=>run('separation_reinstall_runtime')}>Reinstall separation tools</button><button class="button" disabled={!available('separation_remove_runtime')} onclick={()=>confirmation='runtime'}>Remove separation tools…</button></div>
   <p class="small">Reinstalling tools preserves the model. A repair may need downloads again.</p>
   <p class="small">{status?.model_path??'Shared app-data model storage'}</p>
   <div class="management-controls"><button class="button" disabled={!available('separation_reinstall_model')} onclick={()=>run('separation_reinstall_model')}>Reinstall model</button><button class="button" disabled={!available('separation_remove_model')} onclick={()=>confirmation='model'}>Remove model…</button><button class="text-button" disabled={!available('separation_refresh_runtime')} onclick={()=>run('separation_refresh_runtime')}>Check tools again</button></div>
  </details>
 </div>
 {#if confirmation}<div class="confirmation" role="alertdialog" tabindex="-1" aria-label={confirmation==='replace'?'Replace stems confirmation':confirmation==='runtime'?'Remove separation tools confirmation':'Remove model confirmation'}><div class="screen-title"><button class="icon-button" aria-label="Close confirmation" onclick={()=>confirmation=null}><Icon name="close"/></button><h3>{confirmation==='replace'?'Replace stems?':confirmation==='runtime'?'Remove separation tools?':'Remove model?'}</h3></div><p>{confirmation==='replace'?'The existing stems stay available until the new tracks are checked and saved. Matching track gains and mutes are retained.':confirmation==='runtime'?'The separation tools and their download cache will be removed. Model weights, recordings, completed stems, notes and maps remain.':'The downloaded model will be removed. Completed stems, recording, notes and map remain.'}</p><button class="button" disabled={!available(confirmation==='replace'?'separation_start':confirmation==='runtime'?'separation_remove_runtime':'separation_remove_model')} onclick={()=>{run(confirmation==='replace'?'separation_start':confirmation==='runtime'?'separation_remove_runtime':'separation_remove_model');confirmation=null;}}>{confirmation==='replace'?'Separate and replace':confirmation==='runtime'?'Remove separation tools':'Remove model'}</button></div>{/if}
</section>
<style>
 .separation-panel{display:grid;gap:18px;width:100%;max-width:850px}
 .model-card{display:grid;gap:14px;padding:18px;border:1px solid var(--border);border-radius:7px;background:var(--surface-raised,var(--surface))}
 .model-heading{display:flex;align-items:center;justify-content:space-between;gap:12px}.model-heading h3{font-size:18px;font-weight:600;margin:0}.model-state{font-size:12px;color:var(--muted)}
 p{font-size:13px;line-height:1.5;margin:0;color:var(--text-secondary)}.small{font-size:12px;color:var(--muted)}
 .component-status{display:grid;gap:8px;font-size:12px;margin:0}.component-status>div{display:flex;justify-content:space-between;gap:16px}.component-status dt{color:var(--text-secondary)}.component-status dd{margin:0;color:var(--muted)}
 .management-controls{display:flex;flex-wrap:wrap;gap:10px}.button{justify-self:start}.job-controls{display:flex;align-items:center;justify-content:space-between;gap:15px;font-size:12px;color:var(--text-secondary)}
 details{font-size:12px;color:var(--text-secondary)}details[open]{display:grid;gap:12px}summary{cursor:pointer}
 .notice{display:grid;gap:10px}.diagnostics pre{white-space:pre-wrap;overflow-wrap:anywhere;max-height:200px;overflow:auto;font-size:11px}
 .confirmation{display:grid;gap:16px;padding:18px;border:1px solid var(--border-strong);border-radius:7px}
</style>
