// Coalesce note writes while retaining the latest text until persistence succeeds.
export type NotesState={text:string;dirty:boolean;pending:boolean;error:string};
export class NotesSession {
  private state:NotesState={text:'',dirty:false,pending:false,error:''};
  private key:string|null=null;
  private generation=0;
  private timer:ReturnType<typeof setTimeout>|undefined;
  private flight:Promise<void>|null=null;
  private disposed=false;
  constructor(private save:(key:string,text:string)=>Promise<void>,private publish:(state:NotesState)=>void){}
  private emit(){if(!this.disposed)this.publish({...this.state});}
  reset(key:string,text:string){
    clearTimeout(this.timer);this.generation++;this.key=key;
    this.state={text,dirty:false,pending:false,error:''};this.emit();
  }
  change(text:string){
    if(this.disposed||!this.key)return;
    this.state.text=text;this.state.dirty=true;this.state.error='';this.emit();
    clearTimeout(this.timer);this.timer=setTimeout(()=>{void this.flush();},300);
  }
  async flush():Promise<boolean>{
    clearTimeout(this.timer);
    if(this.disposed)return false;
    if(this.flight){await this.flight;return this.state.error?false:this.flush();}
    if(!this.state.dirty||!this.key)return true;
    const key=this.key,text=this.state.text,generation=this.generation;
    this.state.pending=true;this.state.error='';this.emit();
    this.flight=Promise.resolve().then(()=>this.save(key,text)).then(()=>{
      if(generation===this.generation){this.state.dirty=this.state.text!==text;this.state.error='';}
    }).catch(error=>{if(generation===this.generation)this.state.error=String(error);})
      .finally(()=>{this.flight=null;if(generation===this.generation){this.state.pending=false;this.emit();}});
    await this.flight;
    return this.state.error?false:this.state.dirty?this.flush():true;
  }
  dispose(){clearTimeout(this.timer);this.disposed=true;this.generation++;}
}
