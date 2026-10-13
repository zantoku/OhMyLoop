export const STEM_ROLES=['bass','drums','guitar','piano','vocals','other'] as const;
export type StemRole=typeof STEM_ROLES[number];
export type StemTrack={role:StemRole;gain:number;mute:boolean;solo:boolean};
export type StemsSnapshot={reference_token:string;set_id:string|null;tracks:StemTrack[];available:boolean;warning:string|null;attenuation:number|null;model:string|null};
export type StemProgress={reference_token:string;phase:string;completed:number;total:number|null};
export const roleName=(role:StemRole)=>role[0].toUpperCase()+role.slice(1);
export const usesStems=(tracks:StemTrack[])=>tracks.some(track=>track.gain!==1||track.mute||track.solo);
