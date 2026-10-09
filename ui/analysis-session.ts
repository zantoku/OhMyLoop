import { bridge } from './bridge';
import type { AnalysisStatus } from './timing';
export type AnalysisState = { status:AnalysisStatus|null; requesting:boolean; error:string };
export class AnalysisSession {
  private state:AnalysisState={status:null,requesting:false,error:''};
  private owned:number|null=null;
  private polling=false;
  private disposed=false;
  constructor(private read:()=>{key:string;revision:number},private publish:(state:AnalysisState)=>void){}
  private emit(){if(!this.disposed)this.publish({...this.state});}
  async refresh(){
    if(this.polling||this.disposed)return;this.polling=true;
    try{this.state.status=await bridge.analysisStatus();this.emit();}catch(e){this.state.error=String(e);this.emit();}finally{this.polling=false;}
  }
  async start(install=false){
    if(this.state.requesting||this.state.status?.running)return;
    this.state.requesting=true;this.state.error='';this.emit();const before=this.read();
    try{
      const status=install?await bridge.installModel():await bridge.startAnalysis(before.key,before.revision);
      this.owned=status.id;
      if(this.disposed||(!install&&this.read().key!==before.key)){await bridge.cancelAnalysis(status.id);return;}
      this.state.status=status;
    }catch(e){this.state.error=String(e);}finally{this.state.requesting=false;this.emit();}
  }
  async cancel(){if(this.state.status?.running){try{this.state.status=await bridge.cancelAnalysis(this.state.status.id);}catch(e){this.state.error=String(e);}this.emit();}}
  dispose(){this.disposed=true;if(this.owned!==null&&this.state.status?.running)void bridge.cancelAnalysis(this.owned).catch(()=>{});}
}
