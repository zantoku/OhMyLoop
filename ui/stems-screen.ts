// The Stems screen's guided steps (docs/design/stems.md, concept SC): which of
// the three steps is current, the one-line summary of each, and what the pane
// beside them holds. A pure function of the separation status, the song's stem
// set and a manual attachment; no UI or device access.
import type {ProcessingEstimate,SeparationAttempt,SeparationStatus} from './separation';
import {instrumentSummary,processingLabel,type StemProgress,type StemsSnapshot} from './stems';
import {megabytes,stemTools} from './settings';

/** A manual attachment started on the Stems screen. Its progress events carry `id`. */
export type Attachment={id:string;progress:StemProgress|null;running:boolean;error:string};

export type WorkKind='setup'|'separation'|'attach';
export type Work={
 /** The attempt's identity; dismissing a stopped attempt or a finished banner names it. */
 id:string;kind:WorkKind;title:string;progress:StemProgress|null;
 /** The phase shown under the bar: never a file name. */
 phase:string;elapsed:string;running:boolean;
 /** Failed or cancelled; the step list is then open. */
 stopped:boolean;
 /** Stopped on request: shown in neutral tones, without an error. */
 cancelled:boolean;error:string;details:string;
 cancel:{label:string;possible:boolean}|null;
};
export type Pane=
 |{kind:'checking'}
 |{kind:'unsupported';message:string}
 |{kind:'setup';download:string}
 |{kind:'repair';message:string}
 |{kind:'ready';waiting:string|null}
 |{kind:'work';work:Work}
 |{kind:'stopped';work:Work}
 |{kind:'review';banner:{id:string;text:string}|null;waiting:string|null};
export type StepKey='setup'|'separate'|'review';
export type Step={key:StepKey;title:string;state:'done'|'current'|'todo';summary:string};
export type StemsView={steps:Step[];pane:Pane;hasStems:boolean;toolsReady:boolean;
 /** Separation needs more free disk space than there is (D2); Separate waits. */
 lowSpace:{needed:string;free:string}|null;
 /** About how much free space separation needs, when known. */
 needed:string|null;
 /** Tools on disk that no longer verify: repaired in Settings. */
 repair:boolean};

export type StemsViewInput={
 status:SeparationStatus|null;reference:string;snapshot:StemsSnapshot|null;
 attachment:Attachment|null;
 space?:ProcessingEstimate|null;
 /** Stopped attempts and finished banners the player has dismissed. */
 dismissed:ReadonlySet<string>;
};

const TITLES:Record<WorkKind,string>={setup:'Setting up stem separation',separation:'Splitting song into stems',attach:'Attaching stems'};
const STOPPED:Record<WorkKind,string>={setup:'Setup stopped',separation:'Processing stopped',attach:'Attaching stopped'};

export function percent(progress:StemProgress|null):number|null{
 const work=progress?.workflow;
 return work&&work.total>0?Math.floor(Math.max(0,Math.min(1,work.completed/work.total))*100):null;
}
export function elapsed(seconds:number){return `${Math.floor(seconds/60)}:${String(Math.floor(seconds%60)).padStart(2,'0')} elapsed`;}
/** The heavy-work slot's holder, as the player knows it. */
export function heavyWorkName(work:string){return work==='Timing analysis'?'Beat analysis':work;}

function attemptProgress(attempt:SeparationAttempt):StemProgress|null{
 return attempt.progress?{reference_token:attempt.reference_token??'',operation_id:attempt.id,phase:attempt.phase,completed:attempt.progress.completed,total:attempt.progress.total,workflow:attempt.progress.workflow}:null;
}
const ended=(attempt:SeparationAttempt)=>!attempt.running&&(attempt.phase==='Failed'||attempt.phase==='Cancelled');

function attemptWork(kind:'setup'|'separation',attempt:SeparationAttempt):Work{
 const stopped=ended(attempt);
 const cancel=attempt.running?attempt.phase==='Stopping'?{label:'Stopping…',possible:false}:attempt.phase==='Finishing save'?{label:'Finishing save…',possible:false}:{label:'Cancel',possible:attempt.cancellable}:null;
 return {id:attempt.id,kind,title:TITLES[kind],progress:attemptProgress(attempt),phase:stopped?attempt.phase==='Cancelled'?'Cancelled':STOPPED[kind]:attempt.phase,
  elapsed:elapsed(attempt.elapsed_seconds),running:attempt.running,stopped,cancelled:stopped&&attempt.phase==='Cancelled',error:attempt.error??'',details:attempt.details,cancel};
}
function attachWork(attachment:Attachment):Work{
 const stopped=!attachment.running&&!!attachment.error;
 return {id:attachment.id,kind:'attach',title:TITLES.attach,progress:attachment.progress,phase:stopped?STOPPED.attach:processingLabel(attachment.progress),
  elapsed:'',running:attachment.running,stopped,cancelled:false,error:attachment.error,details:'',cancel:null};
}

/** The setup attempt is newer than the song's last separation. */
function setupIsLatest(status:SeparationStatus){
 const {install,job}=status;
 return install.running||(!!install.id&&!job.id)||(install.order??0)>(job.order??0);
}

function currentWork(input:StemsViewInput):Work|null{
 const {status,reference,attachment,dismissed}=input;
 if(attachment?.running)return attachWork(attachment);
 if(status?.install.running)return attemptWork('setup',status.install);
 const job=status?.job;
 const ownJob=!!job&&job.reference_token===reference;
 if(ownJob&&job!.running)return attemptWork('separation',job!);
 if(attachment?.error&&!dismissed.has(attachment.id))return attachWork(attachment);
 if(!status)return null;
 const setup=ended(status.install)&&!dismissed.has(status.install.id)?attemptWork('setup',status.install):null;
 const separation=ownJob&&ended(job!)&&!dismissed.has(job!.id)?attemptWork('separation',job!):null;
 return setup&&separation?setupIsLatest(status)?setup:separation:setup??separation;
}

function finishedBanner(input:StemsViewInput,snapshot:StemsSnapshot):{id:string;text:string}|null{
 const {status,reference,attachment,dismissed}=input;
 const summary=instrumentSummary(snapshot);
 if(attachment&&!attachment.running&&!attachment.error&&!dismissed.has(attachment.id))return {id:attachment.id,text:`Stems attached. ${summary}`};
 const job=status?.job;
 if(job&&!job.running&&job.phase==='Complete'&&job.reference_token===reference&&!dismissed.has(job.id))return {id:job.id,text:`Separation finished. ${summary}`};
 return null;
}

export function stemsView(input:StemsViewInput):StemsView{
 const {status,snapshot}=input;
 const tools=stemTools(status);
 const hasStems=!!snapshot?.set_id;
 const toolsReady=tools.kind==='ready';
 // Tools on disk that no longer verify need Settings; anything missing is a fresh setup.
 const repair=!!status?.runtime.path&&!status.runtime.ready&&status.runtime.supported!==false&&!status.install.running;
 const waiting=status?.heavy_work&&!status.job.running?heavyWorkName(status.heavy_work):null;
 const work=currentWork(input);
 const pane:Pane=work?{kind:work.running?'work':'stopped',work}
  :hasStems?{kind:'review',banner:finishedBanner(input,snapshot!),waiting:toolsReady?waiting:null}
  :tools.kind==='checking'?{kind:'checking'}
  :tools.kind==='unsupported'?{kind:'unsupported',message:status!.runtime.message}
  :repair?{kind:'repair',message:status!.runtime.message}
  :toolsReady?{kind:'ready',waiting}
  :{kind:'setup',download:megabytes(status!.package_bytes)};
 const current:StepKey=work?work.kind==='setup'?'setup':'separate'
  :pane.kind==='review'?'review':pane.kind==='ready'?'separate':'setup';
 const pct=(value:number|null)=>value===null?'':` · ${value} %`;
 const stoppedSummary=(work:Work)=>work.cancelled?'Cancelled · retry':'Stopped · retry';
 const setupSummary=work?.kind==='setup'?work.stopped?stoppedSummary(work):`${work.phase}${pct(percent(work.progress))}`
  :tools.kind==='unsupported'?'Not available on this computer':toolsReady?'Ready · works offline, for every song'
  :tools.kind==='checking'?'Checking…':repair?'Needs repair':`One-time download, ${status?megabytes(status.package_bytes):''}`;
 const separateSummary=work&&work.kind!=='setup'?work.stopped?stoppedSummary(work):`${work.kind==='attach'?'Attaching':'Running'}${pct(percent(work.progress))}`
  :hasStems?`${instrumentSummary(snapshot!,false)}${snapshot!.separated?'':', attached'}`
  :waiting&&toolsReady?`Waits for ${waiting.toLowerCase()}`:'A few minutes · playback pauses';
 const state=(key:StepKey,done:boolean)=>key===current?'current' as const:done?'done' as const:'todo' as const;
 const space=input.space;
 const gb=(bytes:number)=>`${(bytes/1e9).toFixed(1)} GB`;
 const lowSpace=space&&space.available_bytes!==null&&space.required_bytes>space.available_bytes?{needed:gb(space.required_bytes),free:gb(space.available_bytes)}:null;
 return {hasStems,toolsReady,repair,lowSpace,needed:space?gb(space.required_bytes):null,pane,steps:[
  {key:'setup',title:'Set up separation',state:state('setup',toolsReady),summary:setupSummary},
  {key:'separate',title:'Separate this song',state:state('separate',hasStems),summary:separateSummary},
  {key:'review',title:'Review the stems',state:state('review',false),summary:hasStems?'Mix them in Practice':'Once the stems are ready'}
 ]};
}
