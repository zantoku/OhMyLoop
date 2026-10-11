import { describe, expect, it, vi } from 'vitest';
import { ACTIONS, type ActionId } from './actions';
import { ActionDispatcher, type ActionHandlers, type ActionPayload, type ActionState } from './action-dispatcher';

function handlersWith(factory: () => ActionHandlers[ActionId]): ActionHandlers {
  const handlers: Partial<ActionHandlers> = {};
  for (const id of Object.keys(ACTIONS) as ActionId[]) handlers[id] = factory();
  return handlers as ActionHandlers;
}

describe('shared action execution', () => {
  it('checks availability for both keyboard and buttons without executing unavailable actions', () => {
    const handlers = handlersWith(() => vi.fn<(payload: ActionPayload) => void>());
    const state: ActionState = { busy: false, song: true, range: false, sections: false, audio: false, loop_on: false, undo: false, editor: false, help: false, zoom_out: false };
    const dispatcher = new ActionDispatcher(handlers, () => state);
    dispatcher.run('jump_loop_start'); dispatcher.run('play_pause');
    expect(handlers.jump_loop_start).not.toHaveBeenCalled(); expect(handlers.play_pause).not.toHaveBeenCalled();
    state.range = true; dispatcher.run('jump_loop_start'); expect(handlers.jump_loop_start).toHaveBeenCalledOnce();
    state.busy = true; dispatcher.run('jump_song_start'); dispatcher.run('show_shortcuts');
    expect(handlers.jump_song_start).not.toHaveBeenCalled(); expect(handlers.show_shortcuts).toHaveBeenCalledOnce();
  });
  it('requires contextual payloads and preserves them through dispatch', () => {
    const handler = vi.fn<(payload: ActionPayload) => void>();
    const handlers = handlersWith(() => handler);
    const dispatcher = new ActionDispatcher(handlers, () => ({ busy: false, song: true, range: true, sections: true, audio: true, loop_on: true, undo: true, editor: true, help: true, zoom_out: true }));
    dispatcher.run('delete_section'); dispatcher.run('speed_preset', { speed: NaN });
    expect(handler).not.toHaveBeenCalled();
    const section = { id: 'riff', name: 'Riff', start: 1, end: 2 };
    dispatcher.run('select_section', { section }); expect(handler).toHaveBeenLastCalledWith({ section });
  });
});

it('requires library row/sort payloads and scopes library operations',()=>{
  const handler=vi.fn<(payload:ActionPayload)=>void>(),handlers=handlersWith(()=>handler);
  const state:ActionState={context:'library',busy:false,song:false,range:false,sections:false,audio:false,loop_on:false,undo:false,editor:false,help:false,zoom_out:false};const dispatcher=new ActionDispatcher(handlers,()=>state);
  dispatcher.run('library_open');dispatcher.run('library_sort');expect(handler).not.toHaveBeenCalled();
  dispatcher.run('library_open',{songKey:'song'});expect(handler).toHaveBeenLastCalledWith({songKey:'song'});
  dispatcher.run('library_sort',{librarySort:'artist'});expect(handler).toHaveBeenLastCalledWith({librarySort:'artist'});
  state.context='practice';expect(dispatcher.available('library_save')).toBe(false);expect(dispatcher.available('show_library')).toBe(true);
});
