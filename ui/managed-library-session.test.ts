import {beforeEach,expect,it,vi} from 'vitest';
import {ManagedLibrarySession,type ManagedState} from './managed-library-session';
const mock=vi.hoisted(()=>({libraryStatus:vi.fn(),chooseLibrary:vi.fn(),acknowledgeImportNotice:vi.fn(),pickImportSong:vi.fn(),importSong:vi.fn(),openLibrarySong:vi.fn(),deleteLibrarySong:vi.fn()}));
vi.mock('./bridge',()=>({bridge:mock}));
let state:ManagedState;let session:ManagedLibrarySession;
const available={selected:{id:'lib',root:'/music'},available:true,error:null,import_notice_seen:true};
beforeEach(()=>{vi.resetAllMocks();session=new ManagedLibrarySession(value=>{state=value;});mock.libraryStatus.mockResolvedValue(available);});
it('asks for setup before choosing the first recording and remembers cancelled setup',async()=>{
 mock.libraryStatus.mockResolvedValue({...available,selected:null,available:false,import_notice_seen:false});
 await session.beginImport();expect(state.dialog).toBe('setup');expect(mock.pickImportSong).not.toHaveBeenCalled();
 mock.chooseLibrary.mockResolvedValue(null);await session.choose(true);expect(state.dialog).toBe('setup');
 session.close();expect(session.needsImport).toBe(false);
});
it('shows the copy notice once before the picker, with no verification phase',async()=>{
 mock.libraryStatus.mockResolvedValue({...available,import_notice_seen:false});await session.beginImport();expect(state.dialog).toBe('notice');
 mock.pickImportSong.mockResolvedValue({path:'/original.wav',candidates:[]});mock.importSong.mockResolvedValue({transport:{song_key:'new'}});
 await session.continueImport();expect(mock.acknowledgeImportNotice).toHaveBeenCalledOnce();expect(mock.importSong).toHaveBeenCalledWith('/original.wav');expect(state.dialog).toBeNull();
});
it('a dismissed picker never imports',async()=>{mock.pickImportSong.mockResolvedValue(null);await session.beginImport();expect(mock.importSong).not.toHaveBeenCalled();expect(state.dialog).toBeNull();});
it('requires a choice for duplicates and opening existing never imports',async()=>{
 mock.pickImportSong.mockResolvedValue({path:'/song.wav',candidates:[{song_key:'existing'}]});await session.beginImport();expect(state.dialog).toBe('duplicates');expect(mock.importSong).not.toHaveBeenCalled();
 await session.openDuplicate('existing');expect(mock.openLibrarySong).toHaveBeenCalledWith('existing');expect(mock.importSong).not.toHaveBeenCalled();
});
it('separate duplicate imports the selected source directly',async()=>{mock.pickImportSong.mockResolvedValue({path:'/song.wav',candidates:[{song_key:'existing'}]});await session.beginImport();await session.importSelected();expect(mock.importSong).toHaveBeenCalledWith('/song.wav');});
it('does not substitute an unavailable library',async()=>{mock.libraryStatus.mockResolvedValue({...available,available:false,error:'Drive disconnected'});await expect(session.beginImport()).rejects.toThrow('Drive disconnected');expect(mock.chooseLibrary).not.toHaveBeenCalled();expect(mock.pickImportSong).not.toHaveBeenCalled();});
it('deletion requires confirmation and a closed dialog does not delete',async()=>{session.delete('song','Title');expect(mock.deleteLibrarySong).not.toHaveBeenCalled();session.close();expect(state.dialog).toBeNull();session.delete('song','Title');await session.confirmDelete();expect(mock.deleteLibrarySong).toHaveBeenCalledWith('song');});
it('ignores import replies after disposal',async()=>{let finish!:(value:unknown)=>void;mock.pickImportSong.mockReturnValue(new Promise(resolve=>{finish=resolve;}));const pending=session.beginImport();await vi.waitFor(()=>expect(mock.pickImportSong).toHaveBeenCalledOnce());session.dispose();finish({path:'/song.wav',candidates:[]});expect(await pending).toBeNull();expect(mock.importSong).not.toHaveBeenCalled();});
