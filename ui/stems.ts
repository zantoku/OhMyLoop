export const STEM_ROLES=['bass','drums','guitar','piano','vocals','other'] as const;
export type StemRole=typeof STEM_ROLES[number];
export type StemTrack={role:StemRole;gain:number;mute:boolean;solo:boolean};
/** `separated`: made by this app's separation, rather than attached from another tool. */
export type StemsSnapshot={reference_token:string;set_id:string|null;tracks:StemTrack[];available:boolean;warning:string|null;attenuation:number|null;model:string|null;separated:boolean};
export type StemProcessingStep={id:string;label:string;state:'pending'|'active'|'complete'|'skipped';completed:number;total:number|null};
export type StemWorkflowProgress={steps:StemProcessingStep[];completed:number;total:number};
export type StemProgress={reference_token:string;operation_id?:string|null;phase:string;completed:number;total:number|null;workflow?:StemWorkflowProgress|null};
export const processingLabel=(progress:StemProgress|null)=>progress?.workflow?.steps.find(step=>step.state==='active')?.label??'Preparing audio';
export const roleName=(role:StemRole)=>role[0].toUpperCase()+role.slice(1);
const COUNT=['No','One','Two','Three','Four','Five','Six'];
const list=(names:string[])=>names.length<2?names.join(''):`${names.slice(0,-1).join(', ')} and ${names.at(-1)}`;
/**
 * Roles a separated set left out. Separation always yields all six roles and
 * omits only near-silent ones; an attached set may simply have fewer.
 */
export function skippedRoles(snapshot:StemsSnapshot):StemRole[]{
 return snapshot.separated?STEM_ROLES.filter(role=>!snapshot.tracks.some(track=>track.role===role)):[];
}
/** "Five instruments; piano was near-silent and left out." */
export function instrumentSummary(snapshot:StemsSnapshot,withSkipped=true){
 const count=snapshot.tracks.length;
 const kept=`${COUNT[count]??count} instrument${count===1?'':'s'}`;
 const skipped=withSkipped?skippedRoles(snapshot):[];
 if(!skipped.length)return withSkipped?`${kept}.`:kept;
 const names=list(skipped.map(role=>role));
 return `${kept}; ${names} ${skipped.length===1?'was':'were'} near-silent and left out.`;
}
const MODELS:Record<string,string>={'roformer-sw':'Roformer-SW','demucs-6s':'Demucs 6s'};
export const modelName=(model:string|null)=>model?MODELS[model]??model:null;
export const usesStems=(tracks:StemTrack[])=>tracks.some(track=>track.gain!==1||track.mute||track.solo);

// A successful attach reply confirms publication even if its final event arrives late.
export function completedProcessing(progress:StemProgress|null):StemProgress|null{
 if(!progress?.workflow)return progress;
 return {...progress,workflow:{...progress.workflow,completed:progress.workflow.total,steps:progress.workflow.steps.map(step=>({...step,state:step.state==='skipped'?'skipped':'complete',completed:1,total:1}))}};
}
