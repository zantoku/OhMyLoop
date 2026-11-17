import { bridge } from './bridge';
import type { AnalysisStatus, Candidates } from './timing';
export type AnalysisState = { status:AnalysisStatus|null; candidates:Candidates|null; requesting:boolean; error:string };
export class AnalysisSession {
  private state:AnalysisState={status:null,candidates:null,requesting:false,error:''};
  private owned:number|null=null;
  private polling=false;
  private cacheRead=new Set<string>();
  private operation=0;
  private disposed=false;
  constructor(private read:()=>{key:string;revision:number},private publish:(state:AnalysisState)=>void){}
  private accept(status:AnalysisStatus){
    this.state.status=status;
    const key=this.read().key;
    if(this.state.candidates?.reference_token!==key)this.state.candidates=null;
    if(status.result?.reference_token===key)this.state.candidates=status.result;
  }
  private emit(){if(!this.disposed)this.publish({...this.state});}
  async refresh(){
    if(this.polling||this.disposed)return;this.polling=true;const operation=this.operation,keyBefore=this.read().key;
    try{
      const status=await bridge.analysisStatus();
      if(this.disposed||operation!==this.operation||this.read().key!==keyBefore||status.id<(this.state.status?.id??0))return;
      this.accept(status);this.state.error='';this.emit();
      const key=this.read().key;
      if(!this.state.candidates&&!this.cacheRead.has(key)){
        this.cacheRead.add(key);
        const cached=await bridge.readCachedAnalysis(key);
        if(!this.disposed&&operation===this.operation&&this.read().key===key){
          if(cached?.reference_token===key)this.state.candidates=cached;
          this.emit();
        }else this.cacheRead.delete(key);
      }
    }catch(e){if(operation===this.operation&&this.read().key===keyBefore){this.state.error=String(e);this.emit();}}finally{this.polling=false;}
  }
  async start(install=false){
    if(this.state.requesting||this.state.status?.running)return;
    this.operation++;this.state.requesting=true;this.state.error='';this.emit();const before=this.read();
    try{
      const status=install?await bridge.installModel():await bridge.startAnalysis(before.key,before.revision);
      this.operation++;this.owned=status.id;
      if(this.disposed||(!install&&this.read().key!==before.key)){await bridge.cancelAnalysis(status.id);return;}
      this.accept(status);
    }catch(e){this.state.error=String(e);}finally{this.state.requesting=false;this.emit();}
  }
  async cancel(){if(this.state.status?.running){this.operation++;try{this.accept(await bridge.cancelAnalysis(this.state.status.id));this.operation++;}catch(e){this.state.error=String(e);}this.emit();}}
  dispose(){this.disposed=true;if(this.owned!==null&&this.state.status?.running)void bridge.cancelAnalysis(this.owned).catch(()=>{});}
}
