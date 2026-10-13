export const STEM_ROLES=['bass','drums','guitar','piano','vocals','other'] as const;
export type StemRole=typeof STEM_ROLES[number];
export type StemTrack={role:StemRole;gain:number;mute:boolean;solo:boolean};
export type StemsSnapshot={reference_token:string;set_id:string|null;tracks:StemTrack[];available:boolean;warning:string|null;attenuation:number|null;model:string|null};
export type StemProcessingStep={id:string;label:string;state:'pending'|'active'|'complete'|'skipped';completed:number;total:number|null};
export type StemWorkflowProgress={steps:StemProcessingStep[];completed:number;total:number};
export type StemProgress={reference_token:string;operation_id?:string|null;phase:string;completed:number;total:number|null;workflow?:StemWorkflowProgress|null};
export const processingLabel=(progress:StemProgress|null)=>progress?.workflow?.steps.find(step=>step.state==='active')?.label??'Preparing audio';
export const roleName=(role:StemRole)=>role[0].toUpperCase()+role.slice(1);
export const usesStems=(tracks:StemTrack[])=>tracks.some(track=>track.gain!==1||track.mute||track.solo);

// A successful attach reply confirms publication even if its final event arrives late.
export function completedProcessing(progress:StemProgress|null):StemProgress|null{
 if(!progress?.workflow)return progress;
 return {...progress,workflow:{...progress.workflow,completed:progress.workflow.total,steps:progress.workflow.steps.map(step=>({...step,state:step.state==='skipped'?'skipped':'complete',completed:1,total:1}))}};
}
