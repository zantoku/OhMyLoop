import {bridge} from './bridge';
import type {Catalogue,LibraryEntry,SongMetadata} from './library';
/** `error` is a failed list read; `saveError` a failed details save, shown beside the kept edits. */
export type LibraryState={catalogue:Catalogue|null;pending:boolean;error:string;saveError:string};
export class LibrarySession {
  private state:LibraryState={catalogue:null,pending:false,error:'',saveError:''};
  private disposed=false;
  private generation=0;
  constructor(private publish:(state:LibraryState)=>void){}
  private emit(){if(!this.disposed)this.publish({...this.state});}
  invalidate(){this.generation++;this.state={catalogue:null,pending:false,error:'',saveError:''};this.emit();}
  /** Forgets a failed save, e.g. when its edits are cancelled. */
  clearSaveError(){if(this.state.saveError){this.state.saveError='';this.emit();}}
  async refresh(){
    if(this.disposed||this.state.pending)return;
    const generation=this.generation;
    this.state.pending=true;this.state.error='';this.emit();
    try{const catalogue=await bridge.readLibrary();if(!this.disposed&&generation===this.generation)this.state.catalogue=catalogue;}
    catch(e){if(generation===this.generation)this.state.error=String(e);}finally{if(generation===this.generation){this.state.pending=false;this.emit();}}
  }
  async save(key:string,metadata:SongMetadata):Promise<LibraryEntry|null>{
    if(this.disposed||this.state.pending)return null;
    const generation=this.generation;
    this.state.pending=true;this.state.saveError='';this.emit();
    try{
      const catalogue=await bridge.updateLibrarySong(key,metadata);
      if(this.disposed||generation!==this.generation)return null;
      this.state.catalogue=catalogue;
      return catalogue.songs.find(entry=>entry.song_key===key)??null;
    }catch(e){if(generation===this.generation)this.state.saveError=String(e);return null;}
    finally{if(generation===this.generation){this.state.pending=false;this.emit();}}
  }
  dispose(){this.disposed=true;this.generation++;}
}
