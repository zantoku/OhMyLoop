import type {StemsSnapshot,StemTrack,StemRole} from './stems';
export type StemsState={snapshot:StemsSnapshot|null;pending:boolean;error:string};
type Sender=(reference:string,set:string,tracks:StemTrack[],persist:boolean)=>Promise<StemsSnapshot>;
// One in-flight live command, one latest desired value. Saving has its own debounce.
export class StemsSession {
 private generation=0;private disposed=false;private value:StemsSnapshot|null=null;
 private desired:{value:StemsSnapshot;persist:boolean;generation:number}|null=null;
 private running:Promise<void>|null=null;private timer:ReturnType<typeof setTimeout>|undefined;private error='';
 constructor(private send:Sender,private publish:(state:StemsState)=>void){}
 reset(value:StemsSnapshot|null){this.generation++;clearTimeout(this.timer);this.timer=undefined;this.desired=null;this.value=value?structuredClone(value):null;this.error='';this.emit();}
 private emit(){if(!this.disposed)this.publish({snapshot:this.value?structuredClone(this.value):null,pending:!!this.running||!!this.desired,error:this.error});}
 change(role:StemRole,patch:Partial<Omit<StemTrack,'role'>>){
  if(!this.value?.available||!this.value.set_id||this.disposed)return;
  this.value={...this.value,tracks:this.value.tracks.map(track=>track.role===role?{...track,...patch,gain:Number.isFinite(patch.gain)?Math.max(0,Math.min(1,patch.gain!)):track.gain}:track)};
  this.queue(false);clearTimeout(this.timer);this.timer=setTimeout(()=>{this.timer=undefined;this.queue(true);},350);
 }
 private queue(persist:boolean){if(!this.value?.set_id)return;this.desired={value:structuredClone(this.value),persist,generation:this.generation};this.start();this.emit();}
 private start(){if(this.running||!this.desired)return;this.running=this.drain().finally(()=>{this.running=null;this.emit();if(this.desired)this.start();});}
 private async drain(){while(this.desired&&!this.disposed){const request=this.desired;this.desired=null;try{await this.send(request.value.reference_token,request.value.set_id!,request.value.tracks,request.persist);if(request.generation===this.generation)this.error='';}catch(error){if(request.generation===this.generation)this.error=String(error);}this.emit();}}
 async retry(){this.queue(true);await this.flush();}
 async flush(){if(this.timer){clearTimeout(this.timer);this.timer=undefined;this.queue(true);}while(this.running)await this.running;if(this.error)throw new Error(this.error);}
 dispose(){clearTimeout(this.timer);this.disposed=true;this.desired=null;}
}
