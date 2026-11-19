import {afterEach,beforeEach,expect,it,vi} from 'vitest';
import {NOTES_LIMIT_BYTES,NotesSession,SLOW_SAVE_MS,exceedsNotesLimit,notesUnsaved,saveFailureReason,type NotesState} from './notes-session';
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
it('reports a save as slow only after it has run for a while',async()=>{
  let done!:()=>void;const save=vi.fn().mockReturnValueOnce(new Promise<void>(resolve=>{done=resolve;}));let state:NotesState|undefined;const session=new NotesSession(save,value=>{state=value;});session.reset('song','');session.change('Slow');
  const flushed=session.flush();expect(state?.pending).toBe(true);expect(state?.slow).toBe(false);
  await vi.advanceTimersByTimeAsync(SLOW_SAVE_MS-1);expect(state?.slow).toBe(false);await vi.advanceTimersByTimeAsync(1);expect(state?.slow).toBe(true);
  done();expect(await flushed).toBe(true);expect(state).toMatchObject({pending:false,slow:false,dirty:false});session.dispose();
});
it('keeps over-long notes unsent and blocks transitions until they are shortened',async()=>{
  const save=vi.fn().mockResolvedValue(undefined);let state:NotesState|undefined;const session=new NotesSession(save,value=>{state=value;});session.reset('song','Saved');
  const tooLong='€'.repeat(Math.floor(NOTES_LIMIT_BYTES/3)+1);expect(exceedsNotesLimit(tooLong)).toBe(true);expect(exceedsNotesLimit('a'.repeat(NOTES_LIMIT_BYTES))).toBe(false);expect(exceedsNotesLimit('é'.repeat(NOTES_LIMIT_BYTES/2))).toBe(false);expect(exceedsNotesLimit('é'.repeat(NOTES_LIMIT_BYTES/2)+'a')).toBe(true);
  session.change(tooLong);expect(state).toMatchObject({tooLong:true,dirty:true,text:tooLong});expect(notesUnsaved(state!)).toBe(true);
  await vi.advanceTimersByTimeAsync(300);expect(save).not.toHaveBeenCalled();expect(await session.flush()).toBe(false);expect(save).not.toHaveBeenCalled();
  session.change('Short');expect(state?.tooLong).toBe(false);expect(await session.flush()).toBe(true);expect(save).toHaveBeenCalledExactlyOnceWith('song','Short');expect(notesUnsaved(state!)).toBe(false);session.dispose();
});
it('turns a save error into a reason for the drawer',()=>{
  expect(saveFailureReason('Error: Disk full.')).toBe('Disk full');expect(saveFailureReason('Error: Error: No space left on device')).toBe('No space left on device');expect(saveFailureReason('Permission denied')).toBe('Permission denied');
});
