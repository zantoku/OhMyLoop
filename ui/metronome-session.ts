import {bridge} from './bridge';
import type {MeasureMap} from './timing';
export type MetronomeRequest={key:string;enabled:boolean;volume:number;map:MeasureMap|null};
export type MetronomeState={pending:boolean;error:string};
// Coalesce map/level changes while one native request is in flight.
export class MetronomeSession {
  private desired:MetronomeRequest|null=null;
  private signature='';private running=false;private disposed=false;private waiters:Array<()=>void>=[];
  constructor(private publish:(value:MetronomeState)=>void){}
  set(value:MetronomeRequest){
    if(this.disposed)return;
    const signature=JSON.stringify(value);if(signature===this.signature)return;
    this.signature=signature;this.desired=value;void this.drain();
  }
  private async drain(){
    if(this.running)return;this.running=true;
    if(!this.disposed)this.publish({pending:true,error:''});
    let error='';
    while(this.desired){
      const value=this.desired;this.desired=null;
      try{await bridge.metronome(value.key,value.enabled,value.volume,value.map);error='';}catch(e){error=String(e);}
    }
    this.running=false;for(const done of this.waiters.splice(0))done();if(!this.disposed)this.publish({pending:false,error});
  }
  async flush(){if(this.running)await new Promise<void>(resolve=>this.waiters.push(resolve));}
  dispose(){
    this.disposed=true;
    if(this.signature){const value=JSON.parse(this.signature) as MetronomeRequest;this.desired={...value,enabled:false,map:null};void this.drain();}
  }
}
