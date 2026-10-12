import { describe,expect,it } from 'vitest';
import { render,screen } from '@testing-library/svelte';
import AnalysisProgress from './AnalysisProgress.svelte';
import type { AnalysisStatus } from './timing';
const base:AnalysisStatus={id:1,running:true,kind:'analysis',phase:'Finding beats and downbeats',model_path:'/models',model_ready:true,progress:null,elapsed_seconds:125,reference_token:'song',map_revision:0,error:null,result:null};
describe('job progress presentation',()=>{
  it('shows completed analysis segments and live elapsed time',()=>{
    render(AnalysisProgress,{status:{...base,progress:{completed:2,total:8,unit:'segments'}}});
    const progress=screen.getByRole('progressbar') as HTMLProgressElement;
    expect(progress.value).toBe(2);expect(progress.max).toBe(8);
    expect(screen.getByText('2 / 8 segments completed · 25%')).toBeTruthy();expect(screen.getByText('2:05 elapsed')).toBeTruthy();
  });
  it('shows received bytes and indeterminate activity when download size is unknown',()=>{
    render(AnalysisProgress,{status:{...base,kind:'install',phase:'Downloading beat model',progress:{completed:2500000,total:null,unit:'bytes'}}});
    expect(screen.getByRole('progressbar').hasAttribute('value')).toBe(false);expect(screen.getByText('2.5 MB')).toBeTruthy();
  });
  it('does not fabricate a percentage while models load',()=>{
    render(AnalysisProgress,{status:{...base,phase:'Loading analysis models'}});
    expect(screen.getByRole('progressbar').hasAttribute('value')).toBe(false);expect(screen.getByText('Progress for this stage')).toBeTruthy();
  });
});
