import type {StemsSnapshot,StemTrack,StemRole} from './stems';
export type StemsState={snapshot:StemsSnapshot|null;pending:boolean;error:string};
type Sender=(reference:string,set:string,tracks:StemTrack[],persist:boolean)=>Promise<StemsSnapshot>;
const LIVE_INTERVAL_MS=25;
// One in-flight live command and the latest desired value, at most 40 live sends/s.
// The first update is immediate; durable saves have a separate debounce.
export class StemsSession {
 private generation=0;private disposed=false;private value:StemsSnapshot|null=null;
 private desired:{value:StemsSnapshot;persist:boolean;generation:number}|null=null;
 private running:Promise<void>|null=null;private timer:ReturnType<typeof setTimeout>|undefined;
 private liveTimer:ReturnType<typeof setTimeout>|undefined;private nextLiveAt=0;private flushing=false;private error='';
 constructor(private send:Sender,private publish:(state:StemsState)=>void){}
 reset(value:StemsSnapshot|null){this.generation++;clearTimeout(this.timer);clearTimeout(this.liveTimer);this.timer=undefined;this.liveTimer=undefined;this.nextLiveAt=0;this.desired=null;this.value=value?structuredClone(value):null;this.error='';this.emit();}
 private emit(){if(!this.disposed)this.publish({snapshot:this.value?structuredClone(this.value):null,pending:!!this.running||!!this.desired,error:this.error});}
 change(role:StemRole,patch:Partial<Omit<StemTrack,'role'>>){
  if(!this.value?.available||!this.value.set_id||this.disposed)return;
  this.value={...this.value,tracks:this.value.tracks.map(track=>track.role===role?{...track,...patch,gain:Number.isFinite(patch.gain)?Math.max(0,Math.min(1,patch.gain!)):track.gain}:track)};
  this.queue(false);clearTimeout(this.timer);this.timer=setTimeout(()=>{this.timer=undefined;this.queue(true);},350);
 }
 private queue(persist:boolean){if(this.disposed||!this.value?.set_id)return;this.desired={value:structuredClone(this.value),persist,generation:this.generation};this.start();this.emit();}
 private start(){
  if(this.running||!this.desired||this.disposed)return;
  clearTimeout(this.liveTimer);this.liveTimer=undefined;
  const delay=this.nextLiveAt-Date.now();
  if(!this.flushing&&!this.desired.persist&&delay>0){this.liveTimer=setTimeout(()=>{this.liveTimer=undefined;this.start();},delay);return;}
  const request=this.desired;this.desired=null;this.nextLiveAt=Date.now()+LIVE_INTERVAL_MS;
  this.running=this.sendRequest(request).finally(()=>{this.running=null;this.emit();this.start();});
 }
 private async sendRequest(request:NonNullable<StemsSession['desired']>){try{await this.send(request.value.reference_token,request.value.set_id!,request.value.tracks,request.persist);if(request.generation===this.generation)this.error='';}catch(error){if(request.generation===this.generation)this.error=String(error);}this.emit();}
 async retry(){this.queue(true);await this.flush();}
 async flush(){
  if(this.disposed)return;
  this.flushing=true;clearTimeout(this.liveTimer);this.liveTimer=undefined;
  try{if(this.timer){clearTimeout(this.timer);this.timer=undefined;this.queue(true);}this.start();while(this.running||this.desired){if(this.running)await this.running;else this.start();}if(this.error)throw new Error(this.error);}
  finally{this.flushing=false;}
 }
 dispose(){clearTimeout(this.timer);clearTimeout(this.liveTimer);this.timer=undefined;this.liveTimer=undefined;this.disposed=true;this.desired=null;}
}
