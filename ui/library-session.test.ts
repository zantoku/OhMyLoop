import {beforeEach,expect,it,vi} from 'vitest';
import {LibrarySession,type LibraryState} from './library-session';
const api=vi.hoisted(()=>({readLibrary:vi.fn(),updateLibrarySong:vi.fn()}));vi.mock('./bridge',()=>({bridge:api}));
beforeEach(()=>vi.resetAllMocks());
const entry={song_key:'song',metadata:{artist:'Player',title:'Take',album:''},file_location:'/take.wav',date_added:1,last_opened:2};const catalogue={version:1,songs:[entry]};
it('loads and saves metadata through the catalogue boundary',async()=>{
  let state:LibraryState|undefined;api.readLibrary.mockResolvedValue(catalogue);api.updateLibrarySong.mockResolvedValue({...catalogue,songs:[{...entry,metadata:{...entry.metadata,title:'Edited'}}]});
  const session=new LibrarySession(s=>{state=s;});await session.refresh();expect(state?.catalogue).toEqual(catalogue);
  const metadata={...entry.metadata,title:'Edited'};const saved=await session.save('song',metadata);expect(api.updateLibrarySong).toHaveBeenCalledExactlyOnceWith('song',metadata);expect(saved?.metadata.title).toBe('Edited');expect(state?.pending).toBe(false);
});
it('preserves loaded data on failures and supports retry',async()=>{
  let state:LibraryState|undefined;api.readLibrary.mockResolvedValue(catalogue);const session=new LibrarySession(s=>{state=s;});await session.refresh();api.updateLibrarySong.mockRejectedValue(new Error('Disk full'));expect(await session.save('song',entry.metadata)).toBeNull();expect(state?.catalogue).toEqual(catalogue);expect(state?.error).toContain('Disk full');await session.refresh();expect(state?.error).toBe('');
});
it('does not publish late replies after disposal or send concurrent saves',async()=>{
  let finish!:(value:typeof catalogue)=>void;api.readLibrary.mockReturnValue(new Promise(resolve=>{finish=resolve;}));const publish=vi.fn();const session=new LibrarySession(publish);const request=session.refresh();expect(await session.save('song',entry.metadata)).toBeNull();session.dispose();finish(catalogue);await request;expect(publish).toHaveBeenCalledOnce();expect(api.updateLibrarySong).not.toHaveBeenCalled();
});
