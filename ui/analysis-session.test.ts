import {beforeEach,describe,expect,it,vi} from 'vitest';
import {AnalysisSession,type AnalysisState} from './analysis-session';
import type {AnalysisStatus} from './timing';
const api=vi.hoisted(()=>({analysisStatus:vi.fn(),readCachedAnalysis:vi.fn(),installModel:vi.fn(),startAnalysis:vi.fn(),cancelAnalysis:vi.fn()}));vi.mock('./bridge',()=>({bridge:api}));
const status:AnalysisStatus={id:7,running:true,kind:'analysis',phase:'Analyzing',model_path:'/test/models',progress:null,elapsed_seconds:0,model_ready:true,reference_token:'song',map_revision:3,error:null,result:null};
beforeEach(()=>{vi.resetAllMocks();api.readCachedAnalysis.mockResolvedValue(null);});
describe('analysis job orchestration',()=>{
  it('binds requests to source and map revision and exposes cancellation',async()=>{
    let state:AnalysisState|undefined;api.startAnalysis.mockResolvedValue(status);api.cancelAnalysis.mockResolvedValue({...status,phase:'Cancelling'});
    const session=new AnalysisSession(()=>({key:'song',revision:3}),s=>{state=s;});await session.start();expect(api.startAnalysis).toHaveBeenCalledWith('song',3);
    await session.start();expect(api.startAnalysis).toHaveBeenCalledOnce();await session.cancel();expect(api.cancelAnalysis).toHaveBeenCalledWith(7);expect(state?.status?.phase).toBe('Cancelling');
  });
  it('cancels a started job when the screen is destroyed before the start reply arrives',async()=>{
    let resolve!:(value:AnalysisStatus)=>void;api.startAnalysis.mockReturnValue(new Promise(done=>{resolve=done;}));api.cancelAnalysis.mockResolvedValue({...status,running:false});
    const publish=vi.fn();const session=new AnalysisSession(()=>({key:'song',revision:3}),publish);const request=session.start();session.dispose();resolve(status);await request;
    expect(api.cancelAnalysis).toHaveBeenCalledWith(7);expect(publish).toHaveBeenCalledOnce();
  });
  it('reports setup failure without starting inference or modifying a map',async()=>{
    let state:AnalysisState|undefined;api.installModel.mockRejectedValue(new Error('Network unavailable'));
    const session=new AnalysisSession(()=>({key:'song',revision:0}),s=>{state=s;});await session.start(true);expect(state?.error).toContain('Network unavailable');expect(api.startAnalysis).not.toHaveBeenCalled();
  });
});

it('ignores an older status reply that overlaps a new job start',async()=>{
  let finish!:(value:AnalysisStatus)=>void;api.analysisStatus.mockReturnValue(new Promise(done=>{finish=done;}));api.startAnalysis.mockResolvedValue(status);api.cancelAnalysis.mockResolvedValue({...status,running:false});
  const states:AnalysisState[]=[];const session=new AnalysisSession(()=>({key:'song',revision:3}),s=>states.push(s));
  const refresh=session.refresh();await session.start();finish({...status,id:0,running:false});await refresh;
  expect(states.at(-1)?.status?.id).toBe(7);session.dispose();
});

it('ignores a stale status failure after starting a newer job',async()=>{
  let fail!:(reason:Error)=>void;api.analysisStatus.mockReturnValue(new Promise((_,reject)=>{fail=reject;}));api.startAnalysis.mockResolvedValue(status);
  const states:AnalysisState[]=[];const session=new AnalysisSession(()=>({key:'song',revision:3}),s=>states.push(s));
  const refresh=session.refresh();await session.start();fail(new Error('Old request failed'));await refresh;
  expect(states.at(-1)?.status?.id).toBe(7);expect(states.at(-1)?.error).toBe('');
});

const evidence={version:2,reference_token:'song',map_revision:1,rate:48000,frames:576000,model:'test',preprocessing:'test',elapsed_seconds:2,beats:[{frame:0,score:.8,downbeat:true},{frame:48000,score:.8,downbeat:false}]};
it('restores saved evidence once without requiring a new job or matching map revision',async()=>{
  api.analysisStatus.mockResolvedValue({...status,id:0,running:false,result:null});api.readCachedAnalysis.mockResolvedValue(evidence);
  let state:AnalysisState|undefined;const session=new AnalysisSession(()=>({key:'song',revision:9}),s=>{state=s;});
  await session.refresh();await session.refresh();
  expect(state?.candidates).toEqual(evidence);expect(state?.status?.id).toBe(0);expect(api.readCachedAnalysis).toHaveBeenCalledExactlyOnceWith('song');expect(api.startAnalysis).not.toHaveBeenCalled();
  api.startAnalysis.mockResolvedValue(status);await session.start();expect(state?.candidates).toEqual(evidence);
  api.analysisStatus.mockResolvedValue({...status,running:false,result:{...evidence,map_revision:9,beats:[]}});await session.refresh();expect(state?.candidates?.map_revision).toBe(9);
});
it.each(['song change','new job','dispose'])('ignores cached evidence after %s',async(reason)=>{
  let finish!:(value:typeof evidence)=>void;api.analysisStatus.mockResolvedValue({...status,id:0,running:false});api.readCachedAnalysis.mockReturnValue(new Promise(resolve=>{finish=resolve;}));
  let key='song',state:AnalysisState|undefined;const session=new AnalysisSession(()=>({key,revision:3}),s=>{state=s;});const refresh=session.refresh();
  await vi.waitFor(()=>expect(api.readCachedAnalysis).toHaveBeenCalledOnce());
  if(reason==='song change')key='other';else if(reason==='new job'){api.startAnalysis.mockResolvedValue(status);await session.start();}else session.dispose();
  finish(evidence);await refresh;expect(state?.candidates).toBeNull();
});
it('surfaces corrupt cache errors and rejects evidence for a different song',async()=>{
  api.analysisStatus.mockResolvedValue({...status,id:0,running:false});api.readCachedAnalysis.mockRejectedValue(new Error('Unsupported analysis cache'));
  let state:AnalysisState|undefined;const session=new AnalysisSession(()=>({key:'song',revision:3}),s=>{state=s;});await session.refresh();expect(state?.error).toContain('Unsupported analysis cache');expect(state?.candidates).toBeNull();
  api.readCachedAnalysis.mockResolvedValue({...evidence,reference_token:'other'});const other=new AnalysisSession(()=>({key:'song',revision:3}),s=>{state=s;});await other.refresh();expect(state?.candidates).toBeNull();
});

it('rejects cached replies from a previous asset of the same UUID song',async()=>{
  let token='same-song:asset-a:timeline:revision-1:session-a';
  let finish!:(value:typeof evidence)=>void;
  api.analysisStatus.mockResolvedValue({...status,id:0,running:false,reference_token:null});
  api.readCachedAnalysis.mockReturnValue(new Promise(resolve=>{finish=resolve;}));
  let state:AnalysisState|undefined;
  const session=new AnalysisSession(()=>({key:token,revision:3}),s=>{state=s;});
  const request=session.refresh();
  await vi.waitFor(()=>expect(api.readCachedAnalysis).toHaveBeenCalledWith(token));
  const previous=token;token='same-song:asset-b:timeline:revision-2:session-b';
  finish({...evidence,reference_token:previous});await request;
  expect(state?.candidates).toBeNull();
});
