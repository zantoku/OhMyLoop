import {describe,expect,it} from 'vitest';
import {stemsView,type Attachment} from './stems-screen';
import type {SeparationAttempt,SeparationStatus} from './separation';
import type {StemsSnapshot} from './stems';

const idle=():SeparationAttempt=>({id:'',running:false,phase:'Idle',reference_token:null,progress:null,error:null,details:'',elapsed_seconds:0,cancellable:false,result:null});
function status(tools:boolean,model:boolean):SeparationStatus{
 return {package_id:'roformer-sw-v1',profile_id:'cpu2',model_ready:model,model_path:'/models',package_bytes:699416805,runtime:{ready:tools,supported:true,id:'runtime-v1',message:tools?'Ready':'Tools need setup',path:tools?'/python':null},heavy_work:null,install:idle(),job:idle()};
}
const stems=(patch:Partial<StemsSnapshot>={}):StemsSnapshot=>({reference_token:'ref',set_id:'set',available:true,warning:null,attenuation:.7,model:'roformer-sw',separated:true,
 tracks:['bass','drums','guitar','vocals','other'].map(role=>({role:role as StemsSnapshot['tracks'][number]['role'],gain:1,mute:false,solo:false})),...patch});
const view=(s:SeparationStatus|null,snapshot:StemsSnapshot|null=null,attachment:Attachment|null=null,dismissed:string[]=[])=>stemsView({status:s,reference:'ref',snapshot,attachment,dismissed:new Set(dismissed)});
const states=(v:ReturnType<typeof view>)=>v.steps.map(step=>step.state);
const running=(patch:Partial<SeparationAttempt>):SeparationAttempt=>({...idle(),running:true,cancellable:true,...patch});

describe('the Stems guided steps',()=>{
 it('starts at setup with its download size, and never offers separation before the tools are ready',()=>{
  const v=view(status(false,false));
  expect(v.pane).toEqual({kind:'setup',download:'699.4 MB'});
  expect(states(v)).toEqual(['current','todo','todo']);
  expect(v.steps[0].summary).toBe('One-time download, 699.4 MB');
 });
 it('checks first, and explains an unsupported computer',()=>{
  expect(view(null).pane.kind).toBe('checking');
  const s=status(false,false);s.runtime.supported=false;s.runtime.message='This processor is unsupported.';
  const v=view(s);expect(v.pane).toEqual({kind:'unsupported',message:'This processor is unsupported.'});expect(v.steps[0].summary).toBe('Not available on this computer');
 });
 it('sends tools that no longer verify to Settings, and a missing half back to setup',()=>{
  const broken=status(false,true);broken.runtime.path='/python';broken.runtime.message='Python is damaged.';
  expect(view(broken).pane).toEqual({kind:'repair',message:'Python is damaged.'});expect(view(broken).repair).toBe(true);
  expect(view(status(true,false)).pane.kind).toBe('setup');
 });
 it('moves to Separate when the tools are ready, and waits while beat analysis holds the slot',()=>{
  const ready=status(true,true);
  expect(view(ready).pane).toEqual({kind:'ready',waiting:null});expect(states(view(ready))).toEqual(['done','current','todo']);
  ready.heavy_work='Timing analysis';
  expect(view(ready).pane).toEqual({kind:'ready',waiting:'Beat analysis'});expect(view(ready).steps[1].summary).toBe('Waits for beat analysis');
 });
 it('shows running setup with Cancel, and separation with Stopping and Finishing save that can’t be cancelled',()=>{
  const setup=status(false,false);setup.install=running({id:'setup',phase:'Downloading model',elapsed_seconds:108,progress:{stage:'model',completed:1,total:2,workflow:{completed:1,total:2,steps:[]}}});
  const v=view(setup);
  expect(v.pane.kind).toBe('work');
  if(v.pane.kind!=='work')throw new Error();
  expect(v.pane.work).toMatchObject({kind:'setup',title:'Setting up stem separation',phase:'Downloading model',elapsed:'1:48 elapsed',cancel:{label:'Cancel',possible:true}});
  expect(v.steps[0].summary).toBe('Downloading model · 50 %');
  for(const [phase,label] of [['Separating instruments','Cancel'],['Stopping','Stopping…'],['Finishing save','Finishing save…']] as const){
   const s=status(true,true);s.job=running({id:'job',phase,reference_token:'ref',cancellable:phase==='Separating instruments'});
   const w=view(s);if(w.pane.kind!=='work')throw new Error();
   expect(w.pane.work.title).toBe('Splitting song into stems');
   expect(w.pane.work.cancel).toEqual({label,possible:phase==='Separating instruments'});
   expect(states(w)).toEqual(['done','current','todo']);expect(w.steps[1].summary).toBe('Running');
  }
 });
 it('keeps a failed separation with its error until retried, or dismissed when earlier stems remain',()=>{
  const s=status(true,true);s.job={...idle(),id:'job',order:2,reference_token:'ref',phase:'Failed',error:'Not enough memory.',details:'log'};
  const v=view(s);
  expect(v.pane.kind).toBe('stopped');if(v.pane.kind!=='stopped')throw new Error();
  expect(v.pane.work).toMatchObject({phase:'Processing stopped',error:'Not enough memory.',details:'log',stopped:true,cancel:null});
  expect(v.steps[1].summary).toBe('Stopped · retry');
  expect(view(s,stems()).pane.kind).toBe('stopped');
  expect(view(s,stems(),null,['job']).pane.kind).toBe('review');
  // Another song's failure is not this song's.
  s.job.reference_token='other';expect(view(s).pane.kind).toBe('ready');
 });
 it('shows the later of a failed setup and an older separation failure',()=>{
  const s=status(true,false);s.job={...idle(),id:'job',order:1,reference_token:'ref',phase:'Failed',error:'Earlier inference failed'};
  s.install={...idle(),id:'install',order:2,phase:'Failed',error:'Model download failed'};
  const v=view(s);if(v.pane.kind!=='stopped')throw new Error();
  expect(v.pane.work).toMatchObject({kind:'setup',phase:'Setup stopped',error:'Model download failed'});
  expect(states(v)).toEqual(['current','todo','todo']);
 });
 it('reviews an attached set, with a dismissible finished banner naming near-silent instruments',()=>{
  const s=status(true,true);s.job={...idle(),id:'job',reference_token:'ref',phase:'Complete'};
  const v=view(s,stems());
  expect(v.pane).toEqual({kind:'review',waiting:null,banner:{id:'job',text:'Separation finished. Five instruments; piano was near-silent and left out.'}});
  expect(states(v)).toEqual(['done','done','current']);
  expect(v.steps[1].summary).toBe('Five instruments');
  expect(view(s,stems(),null,['job']).pane).toEqual({kind:'review',waiting:null,banner:null});
  // A set from another tool says so, and doesn't guess what it left out.
  const other=view(status(false,false),stems({separated:false,model:'demucs-6s',tracks:stems().tracks.slice(0,4)}));
  expect(other.steps[1].summary).toBe('Four instruments, attached');expect(states(other)).toEqual(['todo','done','current']);
 });
 it('follows a manual attachment, then reports it attached or stopped',()=>{
  const ready=status(true,true);
  const progress={reference_token:'ref',operation_id:'a',phase:'Hash /private/x.wav',completed:1,total:2,workflow:{completed:1,total:4,steps:[{id:'check',label:'Check bass audio',state:'active' as const,completed:1,total:2}]}};
  const v=view(ready,null,{id:'a',progress,running:true,error:''});
  if(v.pane.kind!=='work')throw new Error();
  expect(v.pane.work).toMatchObject({kind:'attach',title:'Attaching stems',phase:'Check bass audio',cancel:null});
  expect(v.steps[1].summary).toBe('Attaching · 25 %');
  expect(view(ready,stems({separated:false}),{id:'a',progress,running:false,error:''}).pane).toMatchObject({kind:'review',banner:{id:'a',text:'Stems attached. Five instruments.'}});
  const failed=view(ready,null,{id:'a',progress,running:false,error:'Invalid provenance'});
  if(failed.pane.kind!=='stopped')throw new Error();
  expect(failed.pane.work).toMatchObject({kind:'attach',phase:'Attaching stopped',error:'Invalid provenance'});
 });
 it('marks a cancelled attempt as cancelled, not failed',()=>{
  const s=status(true,true);s.job={...idle(),id:'job',reference_token:'ref',phase:'Cancelled'};
  const v=view(s);if(v.pane.kind!=='stopped')throw new Error();
  expect(v.pane.work).toMatchObject({stopped:true,cancelled:true,phase:'Cancelled',error:''});
  expect(v.steps[1].summary).toBe('Cancelled · retry');
  s.job.phase='Failed';const f=view(s);if(f.pane.kind!=='stopped')throw new Error();expect(f.pane.work.cancelled).toBe(false);
 });
 it('leaves a stopped attachment once dismissed, even when the song has no stems',()=>{
  const failed:Attachment={id:'a',progress:null,running:false,error:'Invalid provenance'};
  expect(view(status(true,true),null,failed).pane.kind).toBe('stopped');
  expect(view(status(true,true),null,failed,['a']).pane).toEqual({kind:'ready',waiting:null});
 });
 it('warns before starting when the stems’ volume lacks the space separation needs',()=>{
  const space=(available:number|null)=>stemsView({status:status(true,true),reference:'ref',snapshot:null,attachment:null,dismissed:new Set(),space:{required_bytes:1_960_000_000,available_bytes:available}});
  expect(space(500e9)).toMatchObject({lowSpace:null,needed:'2.0 GB'});
  expect(space(1e9).lowSpace).toEqual({needed:'2.0 GB',free:'1.0 GB'});
  expect(space(null).lowSpace).toBeNull();
 });
});
