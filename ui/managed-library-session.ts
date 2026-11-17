import {bridge} from './bridge';
import type {ImportSelection,LibraryStatus,Snapshot} from './types';
export type ManagedDialog='setup'|'settings'|'notice'|'duplicates'|'delete'|null;
export type ManagedState={status:LibraryStatus;dialog:ManagedDialog;selection:ImportSelection|null;pendingImport:boolean;deleteKey:string|null;deleteTitle:string};
export class ManagedLibrarySession {
  private disposed=false;
  private state:ManagedState={status:{selected:null,available:false,error:null,import_notice_seen:false},dialog:null,selection:null,pendingImport:false,deleteKey:null,deleteTitle:''};
  constructor(private publish:(state:ManagedState)=>void){}
  private emit(){if(!this.disposed)this.publish({...this.state});}
  accept(status:LibraryStatus){if(this.disposed)return;this.state.status=status;this.emit();}
  async refresh(){const status=await bridge.libraryStatus();this.accept(status);}
  close(){this.state.dialog=null;this.state.pendingImport=false;this.state.selection=null;this.state.deleteKey=null;this.state.deleteTitle='';this.emit();}
  settings(){this.state.dialog='settings';this.emit();}
  delete(key:string,title:string){this.state.deleteKey=key;this.state.deleteTitle=title;this.state.dialog='delete';this.emit();}
  async beginImport(){
    await this.refresh();if(this.disposed)return null;
    this.state.pendingImport=true;
    if(!this.state.status.selected){this.state.dialog='setup';this.emit();return null;}
    if(!this.state.status.available)throw new Error(this.state.status.error??'Your library is unavailable. Retry or locate it before importing.');
    if(!this.state.status.import_notice_seen){this.state.dialog='notice';this.emit();return null;}
    return this.pick();
  }
  async choose(create:boolean,locate=false):Promise<Snapshot|null>{
    const snapshot=await bridge.chooseLibrary(create,locate);if(this.disposed||!snapshot)return null;
    if(snapshot.library_status)this.accept(snapshot.library_status);else await this.refresh();
    this.state.dialog=null;this.emit();return snapshot;
  }
  get needsImport(){return this.state.pendingImport;}
  async continueImport(){await bridge.acknowledgeImportNotice();if(this.disposed)return null;this.state.status={...this.state.status,import_notice_seen:true};return this.pick();}
  private async pick(){
    this.state.dialog=null;this.emit();const selection=await bridge.pickImportSong();if(this.disposed)return null;
    this.state.selection=selection;this.state.pendingImport=false;
    if(!selection){this.emit();return null;}
    if(selection.candidates.length){this.state.dialog='duplicates';this.emit();return null;}
    return this.importSelected();
  }
  async importSelected(){const path=this.state.selection?.path;if(!path||this.disposed)return null;this.state.dialog=null;this.emit();const snapshot=await bridge.importSong(path);if(this.disposed)return null;this.state.selection=null;this.emit();return snapshot;}
  async openDuplicate(key:string){this.state.dialog=null;this.emit();const snapshot=await bridge.openLibrarySong(key);return this.disposed?null:snapshot;}
  async confirmDelete(){const key=this.state.deleteKey;if(!key||this.disposed)return null;this.state.dialog=null;this.emit();const snapshot=await bridge.deleteLibrarySong(key);return this.disposed?null:snapshot;}
  dispose(){this.disposed=true;}
}
