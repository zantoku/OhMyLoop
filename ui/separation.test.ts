import {describe,expect,it} from 'vitest';
import {finishedAttempt,separationToast,type SeparationAttempt,type SeparationStatus} from './separation';

const idle=():SeparationAttempt=>({id:'',running:false,phase:'Idle',reference_token:null,progress:null,error:null,details:'',elapsed_seconds:0,cancellable:false,result:null});
const status=():SeparationStatus=>({package_id:'roformer-sw-v1',profile_id:'cpu2',model_ready:true,model_path:'/models',package_bytes:699416805,runtime:{ready:true,message:'Ready',path:'/python'},heavy_work:null,install:idle(),job:idle()});
const result={reference_token:'ref',set_id:'set',available:true,warning:null,attenuation:1,model:'roformer-sw',separated:true,
 tracks:(['bass','drums','guitar','vocals','other'] as const).map(role=>({role,gain:1,mute:false,solo:false}))};

describe('the separation toast on other views',()=>{
 it('shows running separation with honest progress and why playback is paused',()=>{
  const s=status();s.job={...idle(),id:'job',running:true,phase:'Separating instruments',reference_token:'ref',progress:{stage:'x',completed:0,total:null,workflow:{completed:58,total:100,steps:[]}}};
  expect(separationToast(s,'ref',true,'')).toEqual({id:'job',kind:'running',title:'Splitting song into stems',progress:.58,detail:'Playback is paused until it finishes.'});
  s.job.phase='Finishing save';expect(separationToast(s,'ref',true,'')?.detail).toMatch(/^Finishing save/);
 });
 it('shows setup with its phase while playback keeps working',()=>{
  const s=status();s.install={...idle(),id:'setup',running:true,phase:'Downloading model'};
  expect(separationToast(s,'ref',false,'')).toMatchObject({kind:'running',title:'Setting up stem separation',progress:null,detail:'Downloading model. Playback keeps working.'});
 });
 it('keeps the outcome until it is seen, and says nothing after a cancellation',()=>{
  const s=status();s.job={...idle(),id:'job',order:1,phase:'Complete',reference_token:'ref',result};
  expect(separationToast(s,'ref',false,'')).toMatchObject({kind:'done',title:'Stems ready',detail:'Five instruments; piano was near-silent and left out.'});
  expect(finishedAttempt(s,'ref')?.attempt.id).toBe('job');
  expect(separationToast(s,'ref',false,'job')).toBeNull();
  expect(separationToast(s,'other',false,'')).toBeNull();
  s.job={...s.job,phase:'Failed',error:'Not enough memory.',result:null};
  expect(separationToast(s,'ref',false,'')).toMatchObject({kind:'failed',title:'Separation stopped',detail:'Not enough memory.'});
  s.job.phase='Cancelled';expect(separationToast(s,'ref',false,'')).toBeNull();
  s.install={...idle(),id:'setup',order:2,phase:'Failed',error:'Download interrupted.'};
  expect(separationToast(s,'ref',false,'')).toMatchObject({id:'setup',kind:'failed',title:'Setup stopped'});
 });
});
