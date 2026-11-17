import type {SeparationState,SeparationStatus} from './separation';
type API={separationStatus:()=>Promise<SeparationStatus>;installSeparationModel:()=>Promise<SeparationStatus>;removeSeparationModel:()=>Promise<SeparationStatus>;reinstallSeparationModel:()=>Promise<SeparationStatus>;reinstallSeparationRuntime:()=>Promise<SeparationStatus>;removeSeparationRuntime:()=>Promise<SeparationStatus>;refreshSeparationRuntime:()=>Promise<SeparationStatus>;startSeparation:(reference:string)=>Promise<SeparationStatus>;cancelSeparation:(id:string)=>Promise<SeparationStatus>};
// Owned by App, not StemsScreen. Poll replies cannot replace a newer command.
export class SeparationSession {
 private value:SeparationStatus|null=null;private pending=false;private pendingInstall=false;private error='';private disposed=false;private epoch=0;private poll:ReturnType<typeof setInterval>|undefined;private polling=false;private completed='';
 constructor(private api:API,private publish:(s:SeparationState)=>void,private finish:(status:SeparationStatus)=>void|Promise<void>){}
 private emit(){if(!this.disposed)this.publish({status:this.value,pending:this.pending,pendingInstall:this.pendingInstall,error:this.error});}
 private async accept(status:SeparationStatus,epoch:number){if(this.disposed||epoch!==this.epoch)return;this.value=status;this.error='';this.emit();if(!status.job.running&&status.job.phase==='Complete'&&status.job.result&&this.completed!==status.job.id){this.completed=status.job.id;await this.finish(status);}}
 async refresh(){if(this.disposed||this.polling)return;this.polling=true;const epoch=this.epoch;try{await this.accept(await this.api.separationStatus(),epoch);}catch(e){if(epoch===this.epoch&&!this.disposed){this.error=String(e);this.emit();}}finally{this.polling=false;}}
 start(){void this.refresh();this.poll=setInterval(()=>{void this.refresh();},250);}
 private async command(work:()=>Promise<SeparationStatus>,install=false){if(this.pending||this.disposed)return;this.pending=true;this.pendingInstall=install;this.error='';const epoch=++this.epoch;this.emit();try{await this.accept(await work(),epoch);}catch(e){if(epoch===this.epoch&&!this.disposed){this.error=String(e);}}finally{if(epoch===this.epoch){this.pending=false;this.pendingInstall=false;this.emit();}}}
 install(){return this.command(this.api.installSeparationModel,true);}
 reinstallModel(){return this.command(this.api.reinstallSeparationModel,true);}
 reinstallRuntime(){return this.command(this.api.reinstallSeparationRuntime,true);}
 removeRuntime(){return this.command(this.api.removeSeparationRuntime);}
 remove(){return this.command(this.api.removeSeparationModel);}
 runtime(){return this.command(this.api.refreshSeparationRuntime);}
 separate(reference:string){return this.command(()=>this.api.startSeparation(reference));}
 cancel(id:string){return this.command(()=>this.api.cancelSeparation(id));}
 dispose(){this.disposed=true;clearInterval(this.poll);this.epoch++;}
}
