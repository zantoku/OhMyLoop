import {describe,it,expect} from 'vitest';
import {render,screen} from '@testing-library/svelte';
import StemProcessingProgress from './StemProcessingProgress.svelte';
import {completedProcessing,type StemProgress} from './stems';
const progress:StemProgress={reference_token:'song',phase:'Decode /private/secret/reference.wav',completed:25,total:100,workflow:{completed:1.25,total:3,steps:[{id:'check',label:'Check reference recording',state:'complete',completed:1,total:1},{id:'bass',label:'Prepare bass playback',state:'active',completed:25,total:100},{id:'save',label:'Save attached stems',state:'pending',completed:0,total:null}]}};
describe('transparent stem processing',()=>{
 it('shows overall task progress and distinct complete, partial and pending steps without paths',()=>{
  render(StemProcessingProgress,{progress,busy:true,operation:'Attaching stems…'});
  expect(screen.getByRole('progressbar',{name:'Overall stem processing'}).getAttribute('value')).toBe(String(1.25/3));
  const list=screen.getByRole('list',{name:'Processing steps'});expect(list.tabIndex).toBe(0);
  const items=list.querySelectorAll('li');expect(items[0].classList.contains('complete')).toBe(true);expect(items[1].getAttribute('aria-current')).toBe('step');expect(items[1].querySelector('.step-circle')!.getAttribute('style')).toContain('90deg');expect(items[2].textContent).toContain('Pending');
  expect(screen.queryByText(/private|reference.wav/)).toBeNull();
 });
 it('keeps an unknown step indeterminate and a failed workflow incomplete',async()=>{
  const unknown={...progress,workflow:{...progress.workflow!,completed:1,steps:progress.workflow!.steps.map(step=>step.id==='bass'?{...step,total:null}:step)}};
  const {rerender}=render(StemProcessingProgress,{progress:unknown,busy:true});expect(screen.getByText('Working')).toBeTruthy();expect(screen.getByText('33%')).toBeTruthy();
  await rerender({progress:unknown,busy:false,failed:true});expect(screen.getByText('Processing stopped')).toBeTruthy();expect(screen.queryByText('Stems ready')).toBeNull();expect(screen.getByText('Stopped')).toBeTruthy();
 });
 it('retains a completed checklist after processing ends',()=>{
  const finished={...progress,workflow:{completed:3,total:3,steps:progress.workflow!.steps.map(step=>({...step,state:'complete' as const,completed:1,total:1}))}};
  render(StemProcessingProgress,{progress:finished,busy:false});expect(screen.getByText('Stems ready')).toBeTruthy();expect(screen.getByText('100%')).toBeTruthy();expect(screen.getAllByText('Complete')).toHaveLength(3);
 });
 it('keeps near-silent omissions visible as skipped after publication',()=>{
  const skipped:StemProgress={...progress,workflow:{completed:1,total:2,steps:[{id:'piano',label:'Skip near-silent piano',state:'skipped',completed:1,total:1},{id:'save',label:'Save attached stems',state:'pending',completed:0,total:null}]}};
  const complete=completedProcessing(skipped)!;
  render(StemProcessingProgress,{progress:complete,busy:false});expect(screen.getByText('Skipped')).toBeTruthy();expect(screen.getByText('Skip near-silent piano')).toBeTruthy();expect(screen.getByText('Stems ready')).toBeTruthy();expect(screen.getByText('100%')).toBeTruthy();
 });

});
