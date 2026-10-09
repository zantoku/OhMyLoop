<script lang="ts">
  import type { Snippet } from 'svelte';
  import type { AnalysisStatus } from './timing';
  let { status, children }:{status:AnalysisStatus;children?:Snippet}=$props();
  const progress=$derived(status.progress);
  const percent=$derived(progress?.total?Math.min(100,Math.floor(progress.completed/progress.total*100)):null);
  const elapsed=$derived(`${Math.floor(status.elapsed_seconds/60)}:${String(Math.floor(status.elapsed_seconds%60)).padStart(2,'0')}`);
  const amount=$derived(progress?.unit==='bytes'
    ? `${(progress.completed/1_000_000).toFixed(1)}${progress.total?` / ${(progress.total/1_000_000).toFixed(1)}`:''} MB`
    : progress?.unit==='segments'?`${progress.completed} / ${progress.total} segments completed`:null);
</script>
<div class="job-progress" aria-label={status.kind==='install'?'Model download progress':'Song analysis progress'}>
  <div class="job-heading"><span class="activity" aria-hidden="true"></span><strong>{status.phase}…</strong><span class="elapsed">{elapsed} elapsed</span></div>
  {#if progress?.total}
    <progress max={progress.total} value={progress.completed} aria-label={`${status.phase} progress`}></progress>
  {:else}
    <progress aria-label={`${status.phase} progress`}></progress>
  {/if}
  <div class="job-detail"><span>{amount??'Progress for this stage'}{percent!==null?` · ${percent}%`:''}</span>{@render children?.()}</div>
</div>
<style>
  .job-progress{padding:12px 14px;margin-bottom:12px;background:#eef2fa;border:1px solid #dbe3f3;border-radius:5px;color:#566e9c;flex-shrink:0;font-size:12px}
  .job-heading{display:flex;align-items:center;gap:8px}.job-heading strong{font-weight:500;flex:1}.elapsed{font-size:11px;white-space:nowrap;color:#7182a0}.activity{width:11px;height:11px;border:2px solid #526fc02b;border-top-color:#526fc0;border-radius:50%;animation:spin 1s linear infinite;flex-shrink:0}
  progress{display:block;width:100%;height:6px;margin:10px 0 6px;border:0;border-radius:3px;overflow:hidden;background:#dbe3f3;accent-color:#6d82bb}progress::-webkit-progress-bar{background:#dbe3f3;border-radius:3px}progress::-webkit-progress-value{background:#6d82bb;border-radius:3px;transition:width .25s}progress:indeterminate{background:linear-gradient(90deg,#dbe3f3 35%,#6d82bb 50%,#dbe3f3 65%);background-size:250% 100%;animation:indeterminate 1.8s ease-in-out infinite}progress:indeterminate::-webkit-progress-bar{background:transparent}.job-detail{display:flex;align-items:center;justify-content:space-between;gap:8px;font-size:11px;min-height:22px}
  @keyframes spin{to{transform:rotate(360deg)}}@keyframes indeterminate{from{background-position:100% 0}to{background-position:0 0}}@media(prefers-reduced-motion:reduce){.activity,progress:indeterminate{animation:none}}
</style>
