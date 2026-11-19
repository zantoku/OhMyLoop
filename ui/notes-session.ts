// Coalesce note writes while retaining the latest text until persistence succeeds.
/** Notes are stored as UTF-8 and bounded like the engine's song record (src/project.rs). */
export const NOTES_LIMIT_BYTES = 1_048_576;
/** A save still running after this long is shown as "Saving…" in the drawer. */
export const SLOW_SAVE_MS = 800;
export type NotesState={text:string;dirty:boolean;pending:boolean;slow:boolean;tooLong:boolean;error:string};
const encoder=new TextEncoder();
export const exceedsNotesLimit=(text:string)=>text.length>NOTES_LIMIT_BYTES/3&&encoder.encode(text).length>NOTES_LIMIT_BYTES;
/** True when notes have unsaved text that cannot be saved as is: a failed save or over-long text. */
export const notesUnsaved=(state:NotesState)=>!!state.error||state.tooLong;
/** "Error: Disk full." → "Disk full", for "Couldn't save your notes: … ." */
export const saveFailureReason=(error:string)=>error.replace(/^(?:Error:\s*)+/,'').replace(/[.\s]+$/,'');
export class NotesSession {
  private state:NotesState={text:'',dirty:false,pending:false,slow:false,tooLong:false,error:''};
  private key:string|null=null;
  private generation=0;
  private timer:ReturnType<typeof setTimeout>|undefined;
  private slowTimer:ReturnType<typeof setTimeout>|undefined;
  private flight:Promise<void>|null=null;
  private disposed=false;
  constructor(private save:(key:string,text:string)=>Promise<void>,private publish:(state:NotesState)=>void){}
  private emit(){if(!this.disposed)this.publish({...this.state});}
  reset(key:string,text:string){
    clearTimeout(this.timer);clearTimeout(this.slowTimer);this.generation++;this.key=key;
    this.state={text,dirty:false,pending:false,slow:false,tooLong:false,error:''};this.emit();
  }
  change(text:string){
    if(this.disposed||!this.key)return;
    this.state.text=text;this.state.dirty=true;this.state.error='';this.state.tooLong=exceedsNotesLimit(text);this.emit();
    // Over-long text is kept in the editor but never sent; the last saved version stays on disk.
    clearTimeout(this.timer);if(!this.state.tooLong)this.timer=setTimeout(()=>{void this.flush();},300);
  }
  /** Saves pending text now. False while it cannot be saved, so callers keep the song open. */
  async flush():Promise<boolean>{
    clearTimeout(this.timer);
    if(this.disposed)return false;
    if(this.flight){await this.flight;return this.state.error?false:this.flush();}
    if(this.state.tooLong)return false;
    if(!this.state.dirty||!this.key)return true;
    const key=this.key,text=this.state.text,generation=this.generation;
    this.state.pending=true;this.state.error='';this.emit();
    this.slowTimer=setTimeout(()=>{if(generation===this.generation&&this.state.pending){this.state.slow=true;this.emit();}},SLOW_SAVE_MS);
    this.flight=Promise.resolve().then(()=>this.save(key,text)).then(()=>{
      if(generation===this.generation){this.state.dirty=this.state.text!==text;this.state.error='';}
    }).catch(error=>{if(generation===this.generation)this.state.error=String(error);})
      .finally(()=>{clearTimeout(this.slowTimer);this.flight=null;if(generation===this.generation){this.state.pending=false;this.state.slow=false;this.emit();}});
    await this.flight;
    return this.state.error?false:this.state.dirty?this.flush():true;
  }
  dispose(){clearTimeout(this.timer);clearTimeout(this.slowTimer);this.disposed=true;this.generation++;}
}
