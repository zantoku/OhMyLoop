import {instrumentSummary,type StemsSnapshot,type StemWorkflowProgress} from './stems';
export type SeparationProgress={stage:string;completed:number;total:number|null;workflow:StemWorkflowProgress|null};
export type SeparationAttempt={order?:number;id:string;running:boolean;phase:string;reference_token:string|null;progress:SeparationProgress|null;error:string|null;details:string;elapsed_seconds:number;cancellable:boolean;result:StemsSnapshot|null};
export type SeparationStatus={package_id:string;profile_id:string;model_ready:boolean;model_path:string;package_bytes:number;runtime:{ready:boolean;message:string;path:string|null;supported?:boolean;id?:string|null};heavy_work:string|null;install:SeparationAttempt;job:SeparationAttempt};
/** Disk space for one separation: what it needs, and what's free where the stems go (null when unknown). */
export type ProcessingEstimate={required_bytes:number;available_bytes:number|null};
export type SeparationState={status:SeparationStatus|null;pending:boolean;pendingInstall?:boolean;error:string};
export const frozenActions=new Set(['play_pause','seek_backward','seek_forward','seek_backward_fine','seek_forward_fine','jump_song_start','jump_loop_start','jump_loop_end','goto_measure','previous_measure','next_measure','previous_section','next_section','select_section','set_loop_start','set_loop_end','toggle_loop','disable_loop','clear_loop','reset_speed','reset_pitch','reset_tuning','speed_down_10','speed_down_5','speed_up_5','speed_up_10','transpose_down','transpose_up','practice_count_in','repeat_count_in','practice_metronome','map_analyze','retry_audio','stem_gain','retry_stem_mix','attach_stems','remove_stems','show_library','open_song','import_separate','managed_create','managed_open','managed_locate','managed_retry','managed_delete','managed_confirm_delete','library_open','library_replace','replace_recording']);
export function separationFrozen(id:string){return frozenActions.has(id)||id.startsWith('mute_')||id.startsWith('solo_')||id.startsWith('map_play')||id.startsWith('map_seek');}

/**
 * The persistent toast on the other views (stems.md decision 3): running work
 * with its progress, then the latest outcome until the player opens Stems
 * (`seen` names it then) or dismisses it. Cancelling needs no notice.
 */
export type SeparationToast={id:string;kind:'running'|'done'|'failed';title:string;detail:string;progress:number|null};
const workFraction=(attempt:SeparationAttempt)=>{const work=attempt.progress?.workflow??attempt.progress;return work?.total?Math.min(1,work.completed/work.total):null;};
/** The attempt that ended last, if it completed or failed. */
export function finishedAttempt(status:SeparationStatus|null,reference:string):{attempt:SeparationAttempt;kind:'setup'|'separation'}|null{
 if(!status||status.install.running||status.job.running)return null;
 const {install,job}=status;
 const setupLatest=!!install.id&&(!job.id||(install.order??0)>(job.order??0));
 const pick=setupLatest?{attempt:install,kind:'setup' as const}:job.reference_token===reference?{attempt:job,kind:'separation' as const}:null;
 if(!pick?.attempt.id)return null;
 return pick.attempt.phase==='Failed'||(pick.kind==='separation'&&pick.attempt.phase==='Complete')?pick:null;
}
export function separationToast(status:SeparationStatus|null,reference:string,separating:boolean,seen:string):SeparationToast|null{
 if(!status)return null;
 const {install,job}=status;
 if(separating)return {id:job.id,kind:'running',title:'Splitting song into stems',progress:workFraction(job),
  detail:job.phase==='Finishing save'?'Finishing save. Playback is available again when it’s done.':'Playback is paused until it finishes.'};
 if(install.running)return {id:install.id,kind:'running',title:'Setting up stem separation',progress:workFraction(install),detail:`${install.phase}. Playback keeps working.`};
 const finished=finishedAttempt(status,reference);
 if(!finished||finished.attempt.id===seen)return null;
 const {attempt,kind}=finished;
 if(attempt.phase==='Complete')return {id:attempt.id,kind:'done',title:'Stems ready',progress:null,detail:attempt.result?instrumentSummary(attempt.result):'Open Stems to see the instruments.'};
 return {id:attempt.id,kind:'failed',title:kind==='setup'?'Setup stopped':'Separation stopped',progress:null,detail:attempt.error??'Open Stems for details.'};
}
