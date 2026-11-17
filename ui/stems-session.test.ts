import {describe,it,expect,vi,afterEach} from 'vitest';
import {StemsSession,type StemsState} from './stems-session';
import {usesStems,type StemsSnapshot} from './stems';
const snapshot=(reference='r'):StemsSnapshot=>({reference_token:reference,set_id:'set',available:true,warning:null,attenuation:.5,model:'roformer-sw',tracks:[{role:'bass',gain:1,mute:false,solo:false},{role:'vocals',gain:1,mute:false,solo:false}]});
afterEach(()=>vi.useRealTimers());
describe('live stem session',()=>{
 it('coalesces dragging behind one native request and saves the latest gain separately',async()=>{
  vi.useFakeTimers();let finish!:()=>void;const send=vi.fn().mockImplementationOnce(()=>new Promise<void>(resolve=>finish=resolve)).mockResolvedValue(snapshot());let state!:StemsState;
  const session=new StemsSession(send,value=>state=value);session.reset(snapshot());session.change('bass',{gain:.9});session.change('bass',{gain:.8});session.change('bass',{gain:.7});
  expect(send).toHaveBeenCalledTimes(1);expect(state.snapshot!.tracks[0].gain).toBe(.7);finish();await Promise.resolve();await Promise.resolve();await vi.advanceTimersByTimeAsync(25);
  expect(send).toHaveBeenCalledTimes(2);expect(send.mock.lastCall![2][0].gain).toBe(.7);expect(send.mock.lastCall![3]).toBe(false);
  await vi.advanceTimersByTimeAsync(350);await session.flush();expect(send.mock.lastCall![3]).toBe(true);expect(send.mock.lastCall![2][0].gain).toBe(.7);session.dispose();
 });
 it('limits continuous pointer updates while keeping live audition and flushing the final value',async()=>{
  vi.useFakeTimers();const send=vi.fn().mockResolvedValue(snapshot());let state!:StemsState;
  const session=new StemsSession(send,value=>state=value);session.reset(snapshot());
  session.change('bass',{gain:.99});await vi.advanceTimersByTimeAsync(0);expect(send).toHaveBeenCalledTimes(1);
  for(let i=0;i<20;i++){session.change('bass',{gain:.9-i/100});await vi.advanceTimersByTimeAsync(1);}
  expect(send).toHaveBeenCalledTimes(1);expect(state.snapshot!.tracks[0].gain).toBeCloseTo(.71);
  await vi.advanceTimersByTimeAsync(5);expect(send).toHaveBeenCalledTimes(2);expect(send.mock.lastCall![2][0].gain).toBeCloseTo(.71);
  session.change('bass',{gain:.3});await session.flush();expect(send).toHaveBeenCalledTimes(3);expect(send.mock.lastCall![2][0].gain).toBe(.3);expect(send.mock.lastCall![3]).toBe(true);
  await vi.advanceTimersByTimeAsync(500);expect(send).toHaveBeenCalledTimes(3);session.dispose();
 });
 it('makes flush and retry harmless after disposal, including a pending save debounce',async()=>{
  const send=vi.fn().mockResolvedValue(snapshot());const session=new StemsSession(send,()=>{});session.reset(snapshot());session.change('bass',{gain:.7});await Promise.resolve();session.dispose();
  await session.flush();await session.retry();expect(send).toHaveBeenCalledTimes(1);
 });
 it('discards stale replies and queued writes when the reference changes',async()=>{
  let reject!:(error:Error)=>void;const send=vi.fn().mockImplementationOnce(()=>new Promise((_,fail)=>reject=fail)).mockResolvedValue(snapshot('new'));let state!:StemsState;
  const session=new StemsSession(send,value=>state=value);session.reset(snapshot());session.change('bass',{mute:true});session.change('bass',{gain:.4});session.reset(snapshot('new'));reject(new Error('Old reference'));await session.flush();
  expect(send).toHaveBeenCalledTimes(1);expect(state.error).toBe('');expect(state.snapshot!.reference_token).toBe('new');expect(state.snapshot!.tracks[0].gain).toBe(1);session.dispose();
 });
 it('keeps unavailable saved controls and never sends their adjustments',()=>{
  const send=vi.fn();let state!:StemsState;const session=new StemsSession(send,value=>state=value);session.reset({...snapshot(),available:false,tracks:[{role:'bass',gain:.6,mute:true,solo:false}]});session.change('bass',{gain:1});expect(send).not.toHaveBeenCalled();expect(state.snapshot!.tracks[0].gain).toBe(.6);session.dispose();
 });
 it('uses original only at neutral controls, with union solo and mute intent selecting stems',()=>{const tracks=snapshot().tracks;expect(usesStems(tracks)).toBe(false);for(const patch of [{gain:.99},{mute:true},{solo:true}])expect(usesStems([{...tracks[0],...patch}])).toBe(true);expect(usesStems([])).toBe(false);});
});
