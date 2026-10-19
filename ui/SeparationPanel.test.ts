import {it,expect,vi} from 'vitest';
import {fireEvent,render,screen,within} from '@testing-library/svelte';
import SeparationPanel from './SeparationPanel.svelte';
import type {SeparationStatus} from './separation';
it('shows later installation failure rather than an older separation failure',()=>{
 const attempt={id:'',running:false,phase:'Idle',reference_token:null,progress:null,error:null,details:'',elapsed_seconds:0,cancellable:false,result:null};
 const status:SeparationStatus={package_id:'roformer-sw-v1',profile_id:'roformer-sw-cpu2-v1',model_ready:false,model_path:'/models',package_bytes:699416805,runtime:{ready:true,message:'Ready',path:'/python'},heavy_work:null,job:{...attempt,id:'job',order:1,reference_token:'ref',phase:'Failed',error:'Earlier inference failed',details:'old details'},install:{...attempt,id:'install',order:2,phase:'Failed',error:'Model download failed',details:'latest network details'}};
 render(SeparationPanel,{state:{status,pending:false,error:''},hasStems:false,reference:'ref',available:()=>true,run:vi.fn(),processingBytes:1950628864});
 expect(screen.getByRole('alert').textContent).toBe('Model download failed');expect(screen.queryByText('Earlier inference failed')).toBeNull();expect(screen.getByText('latest network details')).toBeTruthy();expect(screen.getByRole('button',{name:'Retry setup'})).toBeTruthy();expect(screen.getByText(/Processing storage: approximately 1.95 GB/)).toBeTruthy();
});

const idle=()=>({id:'',running:false,phase:'Idle',reference_token:null,progress:null,error:null,details:'',elapsed_seconds:0,cancellable:false,result:null});
const setupStatus=(tools:boolean,model:boolean):SeparationStatus=>({package_id:'roformer-sw-v1',profile_id:'cpu2',model_ready:model,model_path:'/models',package_bytes:699416805,runtime:{ready:tools,supported:true,id:'runtime-v1',message:tools?'Ready':'Tools need setup',path:tools?'/python':null},heavy_work:null,install:idle(),job:idle()});
function panel(status:SeparationStatus,available=()=>true){const run=vi.fn();const view=render(SeparationPanel,{state:{status,pending:false,error:''},hasStems:false,reference:'ref',available,run});return {...view,run};}

it.each([[false,false],[false,true],[true,false]])('offers unified setup for tools=%s model=%s',async(tools,model)=>{
 const view=panel(setupStatus(tools,model));
 expect(screen.getByRole('button',{name:'Set up stem separation'})).toBeTruthy();
 expect(screen.queryByRole('button',{name:'Separate song'})).toBeNull();
 await fireEvent.click(screen.getByRole('button',{name:'Set up stem separation'}));
 expect(view.run).toHaveBeenCalledWith('separation_install');
 expect(view.run).not.toHaveBeenCalledWith('separation_start');
});
it('shows Ready only for both components and never starts separation automatically',()=>{
 const view=panel(setupStatus(true,true));
 expect(screen.getByRole('button',{name:'Separate song'})).toBeTruthy();
 expect(screen.queryByRole('button',{name:'Set up stem separation'})).toBeNull();
 expect(view.run).not.toHaveBeenCalled();
});
it('keeps unsupported setup disabled and explains runtime availability',()=>{
 const status=setupStatus(false,true);status.runtime.supported=false;status.runtime.message='This processor is unsupported.';panel(status);
 expect((screen.getByRole('button',{name:'Set up stem separation'}) as HTMLButtonElement).disabled).toBe(true);
 expect(screen.getByText('This processor is unsupported.')).toBeTruthy();
});
it('uses shared indeterminate setup progress and preserves ready model information after cancellation',async()=>{
 const status=setupStatus(false,true);status.install={...idle(),id:'setup',order:2,phase:'Installing separation tools',running:true,cancellable:true,progress:{stage:'runtime.dependencies',completed:0,total:null,workflow:{completed:0,total:0,steps:[{id:'runtime.dependencies',label:'Install separation tools',state:'active',completed:0,total:null}]}}};
 const view=panel(status);
 expect(screen.getByText('Install separation tools')).toBeTruthy();
 expect(screen.getByRole('progressbar',{name:'Overall stem processing'}).hasAttribute('value')).toBe(false);
 expect(screen.getByRole('button',{name:'Cancel'})).toBeTruthy();
 // Active setup and its escape hatch precede secondary model information.
 const modelHeading=screen.getByRole('heading',{name:'Roformer-SW'});
 expect(screen.getByRole('progressbar',{name:'Overall stem processing'}).compareDocumentPosition(modelHeading)&Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy();
 expect(screen.getByRole('button',{name:'Cancel'}).compareDocumentPosition(modelHeading)&Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy();
 status.install={...status.install,running:false,phase:'Cancelled',cancellable:false};
 await view.rerender({state:{status:{...status},pending:false,error:''},hasStems:false,reference:'ref',available:()=>true,run:view.run});
 expect(screen.getByRole('button',{name:'Retry setup'})).toBeTruthy();
 expect(screen.getAllByText('Ready').length).toBe(1);
 expect(view.run).not.toHaveBeenCalled();
});
it('confirms removal of tools with leading close control and retains models/data',async()=>{
 const view=panel(setupStatus(true,true));
 await fireEvent.click(screen.getByRole('button',{name:'Remove separation tools…'}));
 const dialog=screen.getByRole('alertdialog',{name:'Remove separation tools confirmation'});
 expect(within(dialog).getByRole('button',{name:'Close confirmation'})).toBeTruthy();
 expect(within(dialog).getByText(/Model weights, recordings, completed stems, notes and maps remain/)).toBeTruthy();
 await fireEvent.click(within(dialog).getByRole('button',{name:'Remove separation tools'}));
 expect(view.run).toHaveBeenCalledWith('separation_remove_runtime');
 expect(view.run).not.toHaveBeenCalledWith('separation_remove_model');
});
it('disables management while relevant assets are in use',()=>{
 panel(setupStatus(true,true),()=>false);
 for(const name of ['Reinstall separation tools','Remove separation tools…','Reinstall model','Remove model…'])expect((screen.getByRole('button',{name}) as HTMLButtonElement).disabled).toBe(true);
});
