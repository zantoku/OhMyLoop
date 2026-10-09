import {beforeEach,describe,expect,it,vi} from 'vitest';
import {AnalysisSession,type AnalysisState} from './analysis-session';
import type {AnalysisStatus} from './timing';
const api=vi.hoisted(()=>({analysisStatus:vi.fn(),installModel:vi.fn(),startAnalysis:vi.fn(),cancelAnalysis:vi.fn()}));vi.mock('./bridge',()=>({bridge:api}));
const status:AnalysisStatus={id:7,running:true,kind:'analysis',phase:'Analyzing',model_path:'/test/models',progress:null,elapsed_seconds:0,model_ready:true,song_key:'song',map_revision:3,error:null,result:null};
beforeEach(()=>vi.resetAllMocks());
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
