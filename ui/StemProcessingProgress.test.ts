import {describe,it,expect,vi} from 'vitest';
import {fireEvent,render,screen} from '@testing-library/svelte';
import StemProcessingProgress from './StemProcessingProgress.svelte';
import {completedProcessing,type StemProgress} from './stems';
import type {Work} from './stems-screen';
import {ownsKey} from './key-routing';
import {NAVIGATION_KEYS} from './keymap';
const progress:StemProgress={reference_token:'song',phase:'Decode /private/secret/reference.wav',completed:25,total:100,workflow:{completed:1.25,total:3,steps:[{id:'check',label:'Check reference recording',state:'complete',completed:1,total:1},{id:'bass',label:'Prepare bass playback',state:'active',completed:25,total:100},{id:'save',label:'Save attached stems',state:'pending',completed:0,total:null}]}};
const work=(patch:Partial<Work>={}):Work=>({id:'job',kind:'separation',title:'Splitting song into stems',progress,phase:'Separating instruments',elapsed:'3:12 elapsed',running:true,stopped:false,cancelled:false,error:'',details:'',cancel:{label:'Cancel',possible:true},...patch});
describe('transparent stem processing',()=>{
 it('shows the fixed title, overall progress, phase and elapsed time, with the steps behind Show steps',async()=>{
  render(StemProcessingProgress,{work:work()});
  expect(screen.getByRole('heading',{name:'Splitting song into stems'}).getAttribute('tabindex')).toBe('-1');
  expect(screen.getByRole('status').textContent).toBe('Separating instruments');
  expect(screen.getByText('3:12 elapsed')).toBeTruthy();
  expect(screen.getByText('41 %')).toBeTruthy();
  expect(screen.getByRole('progressbar',{name:'Splitting song into stems'}).getAttribute('value')).toBe('41');
  expect(screen.queryByRole('list',{name:'Processing steps'})).toBeNull();
  await fireEvent.click(screen.getByRole('button',{name:/Show steps/}));
  expect(screen.getByRole('button',{name:/Hide steps/}).getAttribute('aria-expanded')).toBe('true');
  const list=screen.getByRole('list',{name:'Processing steps'});expect(list.tabIndex).toBe(0);
  const items=list.querySelectorAll('li');expect(items[0].classList.contains('complete')).toBe(true);expect(items[1].getAttribute('aria-current')).toBe('step');expect(items[1].querySelector('.circle')!.getAttribute('style')).toContain('90deg');expect(items[1].textContent).toContain('25 %');expect(items[2].textContent).toContain('Pending');
  expect(screen.queryByText(/private|reference.wav/)).toBeNull();
 });
 it('keeps unmeasured work indeterminate, with no percentage',async()=>{
  const unknown={...progress,workflow:{completed:1,total:0,steps:progress.workflow!.steps.map(step=>step.id==='bass'?{...step,total:null}:step)}};
  render(StemProcessingProgress,{work:work({progress:unknown})});
  expect(screen.getByRole('progressbar',{name:'Splitting song into stems'}).hasAttribute('value')).toBe(false);
  expect(screen.queryByText(/%/)).toBeNull();
  await fireEvent.click(screen.getByRole('button',{name:/Show steps/}));expect(screen.getByText('Working')).toBeTruthy();
 });
 it('opens the steps when work stops, leaves them incomplete and offers the details to copy',async()=>{
  const writeText=vi.fn().mockResolvedValue(undefined);Object.assign(navigator,{clipboard:{writeText}});
  render(StemProcessingProgress,{work:work({running:false,stopped:true,phase:'Processing stopped',error:'Not enough memory to finish.',details:'worker log',cancel:null})});
  expect(screen.queryByRole('button',{name:/Show steps/})).toBeNull();
  expect(screen.getByRole('list',{name:'Processing steps'})).toBeTruthy();expect(screen.getByText('Stopped')).toBeTruthy();
  expect(screen.getByRole('alert').textContent).toContain('Not enough memory to finish.');
  expect(screen.queryByText('worker log')).toBeNull();
  await fireEvent.click(screen.getByRole('button',{name:'Copy details'}));
  expect(writeText).toHaveBeenCalledWith('worker log');
 });
 it('shows a cancellation in neutral tones, with the steps open and no error',()=>{
  const {container}=render(StemProcessingProgress,{work:work({running:false,stopped:true,cancelled:true,phase:'Cancelled',cancel:null})});
  expect(screen.queryByRole('alert')).toBeNull();expect(screen.queryByText(/stopped/i)).toBeNull();
  expect(screen.getByRole('list',{name:'Processing steps'})).toBeTruthy();expect(screen.getByText('Cancelled',{selector:'.step-status'})).toBeTruthy();
  expect(container.querySelector('.work')!.classList.contains('failed')).toBe(false);
 });
 it('keeps near-silent omissions visible as skipped after publication',async()=>{
  const skipped:StemProgress={...progress,workflow:{completed:1,total:2,steps:[{id:'piano',label:'Skip near-silent piano',state:'skipped',completed:1,total:1},{id:'save',label:'Save attached stems',state:'pending',completed:0,total:null}]}};
  render(StemProcessingProgress,{work:work({progress:completedProcessing(skipped)})});
  await fireEvent.click(screen.getByRole('button',{name:/Show steps/}));
  expect(screen.getByText('Skipped')).toBeTruthy();expect(screen.getByText('Skip near-silent piano')).toBeTruthy();expect(screen.getByText('100 %')).toBeTruthy();
 });
 it('lets the step list own only its scrolling keys; letters, Space and Esc reach the shortcuts',async()=>{
  render(StemProcessingProgress,{work:work({running:false,stopped:true,cancel:null})});
  const list=screen.getByRole('list',{name:'Processing steps'});
  for(const key of ['ArrowUp','ArrowDown','PageUp','PageDown','Home','End'])expect(ownsKey(list,key,'keyboard'),key).toBe(true);
  for(const key of ['ArrowLeft','ArrowRight',' ','Enter','Escape','n','l'])expect(ownsKey(list,key,'keyboard'),key).toBe(false);
  expect(NAVIGATION_KEYS).toContain('ArrowDown');
 });
});
