import {bridge} from './bridge';
import type {Catalogue,LibraryEntry,SongMetadata} from './library';
export type LibraryState={catalogue:Catalogue|null;pending:boolean;error:string};
export class LibrarySession {
  private state:LibraryState={catalogue:null,pending:false,error:''};
  private disposed=false;
  constructor(private publish:(state:LibraryState)=>void){}
  private emit(){if(!this.disposed)this.publish({...this.state});}
  async refresh(){
    if(this.disposed||this.state.pending)return;
    this.state.pending=true;this.state.error='';this.emit();
    try{const catalogue=await bridge.readLibrary();if(!this.disposed)this.state.catalogue=catalogue;}
    catch(e){this.state.error=String(e);}finally{this.state.pending=false;this.emit();}
  }
  async save(key:string,metadata:SongMetadata):Promise<LibraryEntry|null>{
    if(this.disposed||this.state.pending)return null;
    this.state.pending=true;this.state.error='';this.emit();
    try{
      const catalogue=await bridge.updateLibrarySong(key,metadata);
      if(this.disposed)return null;
      this.state.catalogue=catalogue;
      return catalogue.songs.find(entry=>entry.song_key===key)??null;
    }catch(e){this.state.error=String(e);return null;}
    finally{this.state.pending=false;this.emit();}
  }
  dispose(){this.disposed=true;}
}
