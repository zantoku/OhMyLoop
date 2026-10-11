import {afterEach,beforeEach,expect,it,vi} from 'vitest';
import {NotesSession,type NotesState} from './notes-session';
beforeEach(()=>vi.useFakeTimers());afterEach(()=>vi.useRealTimers());
it('coalesces typing and flushes immediately before navigation',async()=>{
  const save=vi.fn().mockResolvedValue(undefined);let state:NotesState|undefined;const session=new NotesSession(save,value=>{state=value;});session.reset('song','');session.change('First');session.change('  First\nSecond  ');expect(save).not.toHaveBeenCalled();await session.flush();expect(save).toHaveBeenCalledExactlyOnceWith('song','  First\nSecond  ');expect(state?.dirty).toBe(false);session.dispose();
});
it('autosaves after the delay and writes the latest input received during a save',async()=>{
  let done!:(value:void)=>void;const save=vi.fn().mockReturnValueOnce(new Promise(resolve=>{done=resolve;})).mockResolvedValue(undefined);let state:NotesState|undefined;const session=new NotesSession(save,value=>{state=value;});session.reset('song','');session.change('First');await vi.advanceTimersByTimeAsync(300);expect(save).toHaveBeenCalledOnce();session.change('Newest');done();await session.flush();expect(save).toHaveBeenLastCalledWith('song','Newest');expect(state?.text).toBe('Newest');expect(state?.dirty).toBe(false);session.dispose();
});
it('retains failed text for retry and refuses to flush successfully until saved',async()=>{
  const save=vi.fn().mockRejectedValueOnce(new Error('Disk full')).mockResolvedValue(undefined);let state:NotesState|undefined;const session=new NotesSession(save,value=>{state=value;});session.reset('song','Old');session.change('New');expect(await session.flush()).toBe(false);expect(state?.text).toBe('New');expect(state?.dirty).toBe(true);expect(state?.error).toContain('Disk full');expect(await session.flush()).toBe(true);expect(state?.error).toBe('');session.dispose();
});
it('does not save or publish delayed work after disposal',async()=>{
  const save=vi.fn(),publish=vi.fn();const session=new NotesSession(save,publish);session.reset('song','');session.change('New');session.dispose();await vi.advanceTimersByTimeAsync(300);expect(save).not.toHaveBeenCalled();expect(publish).toHaveBeenCalledTimes(2);
});
