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
  .job-progress{padding:12px 14px;margin-bottom:12px;background:var(--hover);border:1px solid var(--progress-track);border-radius:5px;color:var(--link);flex-shrink:0;font-size:12px}
  .job-heading{display:flex;align-items:center;gap:8px}.job-heading strong{font-weight:500;flex:1}.elapsed{font-size:11px;white-space:nowrap;color:var(--muted)}.activity{width:11px;height:11px;border:2px solid var(--blue-wash);border-top-color:var(--blue);border-radius:50%;animation:spin 1s linear infinite;flex-shrink:0}
  progress{display:block;width:100%;height:6px;margin:10px 0 6px;border:0;border-radius:3px;overflow:hidden;background:var(--progress-track);accent-color:var(--blue)}progress::-webkit-progress-bar{background:var(--progress-track);border-radius:3px}progress::-webkit-progress-value{background:var(--blue);border-radius:3px;transition:width .25s}progress:indeterminate{background:linear-gradient(90deg,var(--progress-track) 35%,var(--blue) 50%,var(--progress-track) 65%);background-size:250% 100%;animation:indeterminate 1.8s ease-in-out infinite}progress:indeterminate::-webkit-progress-bar{background:transparent}.job-detail{display:flex;align-items:center;justify-content:space-between;gap:8px;font-size:11px;min-height:22px}
  @keyframes spin{to{transform:rotate(360deg)}}@keyframes indeterminate{from{background-position:100% 0}to{background-position:0 0}}@media(prefers-reduced-motion:reduce){.activity,progress:indeterminate{animation:none}}
</style>
