import {beforeEach,expect,it,vi} from 'vitest';
import {CountInSession} from './count-in-session';
const api=vi.hoisted(()=>({countIn:vi.fn()}));vi.mock('./bridge',()=>({bridge:api}));
beforeEach(()=>{vi.resetAllMocks();api.countIn.mockResolvedValue({});});
const request={key:'song',enabled:true,repeat:false,volume:.5};
it('coalesces settings and makes Play wait for the latest configuration',async()=>{
  let finish!:()=>void;api.countIn.mockReturnValueOnce(new Promise<void>(resolve=>{finish=resolve;}));
  const publish=vi.fn(),session=new CountInSession(publish);
  session.set(request);session.set({...request,repeat:true});session.set({...request,repeat:true,volume:.8});
  let flushed=false;const pending=session.flush().then(()=>{flushed=true;});
  await Promise.resolve();expect(flushed).toBe(false);expect(api.countIn).toHaveBeenCalledOnce();
  finish();await pending;
  expect(api.countIn).toHaveBeenCalledTimes(2);expect(api.countIn.mock.lastCall).toEqual(['song',true,true,.8]);
  expect(publish.mock.lastCall).toEqual([{pending:false,error:''}]);
});
it('surfaces native errors to Play and recovers on a settings change',async()=>{
  api.countIn.mockRejectedValueOnce(new Error('Audio stopped'));
  const session=new CountInSession(vi.fn());session.set(request);
  await expect(session.flush()).rejects.toThrow('Audio stopped');
  session.set({...request,enabled:false});await expect(session.flush()).resolves.toBeUndefined();
});
it('disables count-in after an in-flight request when disposed',async()=>{
  let finish!:()=>void;api.countIn.mockReturnValueOnce(new Promise<void>(resolve=>{finish=resolve;}));
  const publish=vi.fn(),session=new CountInSession(publish);session.set(request);session.dispose();finish();await session.flush();
  expect(api.countIn.mock.lastCall).toEqual(['song',false,false,.5]);expect(publish).toHaveBeenCalledOnce();
});
