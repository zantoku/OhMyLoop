import {bridge} from './bridge';
export type CountInRequest={key:string;enabled:boolean;repeat:boolean;volume:number};
export type CountInState={pending:boolean;error:string};
// Serialize changes and let Play wait for the latest native configuration.
export class CountInSession {
  private desired:CountInRequest|null=null;
  private last:CountInRequest|null=null;
  private signature='';private disposed=false;
  private running:Promise<void>|null=null;private error='';
  constructor(private publish:(value:CountInState)=>void){}
  set(value:CountInRequest){
    if(this.disposed)return;
    const signature=JSON.stringify(value);if(signature===this.signature)return;
    this.signature=signature;this.last=value;this.desired=value;this.start();
  }
  private start(){if(!this.running)this.running=this.drain().finally(()=>{this.running=null;if(this.desired)this.start();});}
  private async drain(){
    if(!this.disposed)this.publish({pending:true,error:''});
    while(this.desired){
      const value=this.desired;this.desired=null;
      try{await bridge.countIn(value.key,value.enabled,value.repeat,value.volume);this.error='';}catch(e){this.error=String(e);}
    }
    if(!this.disposed)this.publish({pending:false,error:this.error});
  }
  async flush(){
    while(this.running)await this.running;
    if(this.error)throw new Error(this.error);
  }
  dispose(){
    this.disposed=true;
    if(this.last){this.desired={...this.last,enabled:false};this.start();}
  }
}
