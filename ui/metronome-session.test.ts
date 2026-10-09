import {beforeEach,expect,it,vi} from 'vitest';
import {MetronomeSession,type MetronomeRequest} from './metronome-session';
import {timingFixture} from './timing-fixture';
const api=vi.hoisted(()=>({metronome:vi.fn()}));vi.mock('./bridge',()=>({bridge:api}));
const request=():MetronomeRequest=>({key:'song',enabled:true,volume:.5,map:timingFixture().map});
const flush=async()=>{await Promise.resolve();await Promise.resolve();};
beforeEach(()=>{vi.resetAllMocks();api.metronome.mockResolvedValue({});});
it('compares map content rather than revision and coalesces pending updates',async()=>{
  let finish!:()=>void;api.metronome.mockReturnValueOnce(new Promise<void>(done=>{finish=done;}));
  const session=new MetronomeSession(vi.fn());session.set(request());session.set(request());
  const changed=request();changed.map!.boundaries[1].frame=3500;session.set({...changed,volume:.6});session.set({...changed,volume:.7});
  expect(api.metronome).toHaveBeenCalledOnce();finish();await flush();
  expect(api.metronome).toHaveBeenCalledTimes(2);expect(api.metronome.mock.lastCall).toEqual(['song',true,.7,changed.map]);
});
it('disables after a late enable reply when disposed and publishes no late state',async()=>{
  let finish!:()=>void;api.metronome.mockReturnValueOnce(new Promise<void>(done=>{finish=done;}));
  const publish=vi.fn(),session=new MetronomeSession(publish);session.set(request());session.dispose();finish();await flush();
  expect(api.metronome.mock.lastCall).toEqual(['song',false,.5,null]);expect(publish).toHaveBeenCalledOnce();
});
it('clears an earlier failure after a newer request succeeds',async()=>{
  let fail!:(error:Error)=>void;api.metronome.mockReturnValueOnce(new Promise((_,reject)=>{fail=reject;}));
  const publish=vi.fn(),session=new MetronomeSession(publish);session.set(request());session.set({...request(),volume:.8});
  fail(new Error('Old request failed'));await flush();expect(publish.mock.lastCall?.[0]).toEqual({pending:false,error:''});
});
