import { beforeEach,describe,expect,it,vi } from 'vitest';
import { MappingSession,type MappingState } from './mapping-session';
import { timingFixture } from './timing-fixture';
import type { MeasureMap } from './timing';
const api=vi.hoisted(()=>({previewMap:vi.fn(),mapView:vi.fn()}));vi.mock('./bridge',()=>({bridge:api}));
beforeEach(()=>vi.resetAllMocks());
function setup(){let current:{key:string;map:MeasureMap|null}={key:'song',map:null};let state:MappingState;
  const commit=vi.fn(async(value)=>{current={...current,map:structuredClone(value.map)};});
  const session=new MappingSession(()=>current,commit,value=>{state=value;});
  return {session,commit,get state(){return state;},get current(){return current;},set current(value){current=value;}};
}
const build={type:'build' as const,start:0,end:8000,count:2,meter:timingFixture().map.measures[0].meter};
describe('mapping edit orchestration',()=>{
  it('previews without saving, then applies and keeps undo/redo with increasing revisions',async()=>{
    const s=setup();api.previewMap.mockResolvedValue(timingFixture());await s.session.edit(build);
    expect(s.commit).not.toHaveBeenCalled();expect(s.state.preview?.view.measures).toHaveLength(2);
    await s.session.apply();expect(s.current.map?.measures).toHaveLength(2);expect(s.state.undo).toBe(1);
    api.mapView.mockImplementation(async(map)=>({...timingFixture().view,revision:map.revision,measures:map.measures.length?timingFixture().view.measures:[]}));
    await s.session.history('undo');expect(s.current.map?.measures).toHaveLength(0);expect(s.current.map?.revision).toBe(2);expect(s.current.map?.next_id).toBe(6);
    await s.session.history('redo');expect(s.current.map?.measures).toHaveLength(2);expect(s.current.map?.revision).toBe(3);
  });
  it('rejects stale replies, overlapping requests and failed domain edits without changing the map',async()=>{
    const s=setup();let resolve!:(value:ReturnType<typeof timingFixture>)=>void;
    api.previewMap.mockReturnValue(new Promise(done=>{resolve=done;}));const request=s.session.edit(build);
    await s.session.edit(build);expect(api.previewMap).toHaveBeenCalledOnce();s.current={key:'other',map:null};resolve(timingFixture());await request;
    expect(s.commit).not.toHaveBeenCalled();expect(s.state.preview).toBeNull();expect(s.state.error).toContain('changed');
    api.previewMap.mockRejectedValue(new Error('Protected measure'));await s.session.edit(build,false);
    expect(s.current.map).toBeNull();expect(s.state.error).toContain('Protected');
  });
  it('does not apply a preview over later accepted edits and preserves history when saving fails',async()=>{
    const s=setup();api.previewMap.mockResolvedValue(timingFixture());await s.session.edit(build);s.current={key:'song',map:{...timingFixture().map,revision:4}};
    await s.session.apply();expect(s.commit).not.toHaveBeenCalled();expect(s.state.error).toContain('changed');
    s.current={key:'song',map:null};s.commit.mockRejectedValueOnce(new Error('Disk full'));await s.session.edit(build,false);
    expect(s.state.undo).toBe(0);expect(s.state.error).toContain('Disk full');
  });
});
