import {it,expect,vi} from 'vitest';
import {render,screen} from '@testing-library/svelte';
import SeparationPanel from './SeparationPanel.svelte';
import type {SeparationStatus} from './separation';
it('shows later installation failure rather than an older separation failure',()=>{
 const attempt={id:'',running:false,phase:'Idle',reference_token:null,progress:null,error:null,details:'',elapsed_seconds:0,cancellable:false,result:null};
 const status:SeparationStatus={package_id:'roformer-sw-v1',profile_id:'roformer-sw-cpu2-v1',model_ready:false,model_path:'/models',package_bytes:699416805,runtime:{ready:true,message:'Ready',path:'/python'},heavy_work:null,job:{...attempt,id:'job',order:1,reference_token:'ref',phase:'Failed',error:'Earlier inference failed',details:'old details'},install:{...attempt,id:'install',order:2,phase:'Failed',error:'Model download failed',details:'latest network details'}};
 render(SeparationPanel,{state:{status,pending:false,error:''},hasStems:false,reference:'ref',available:()=>true,run:vi.fn(),processingBytes:1950628864});
 expect(screen.getByRole('alert').textContent).toBe('Model download failed');expect(screen.queryByText('Earlier inference failed')).toBeNull();expect(screen.getByText('latest network details')).toBeTruthy();expect(screen.getByRole('button',{name:'Retry download'})).toBeTruthy();expect(screen.getByText(/Processing storage: approximately 1.95 GB/)).toBeTruthy();
});
