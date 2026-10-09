import {describe,expect,it,vi} from 'vitest';
import {fireEvent,render,screen} from '@testing-library/svelte';
import MapTimeline from './MapTimeline.svelte';
import {timingFixture} from './timing-fixture';
async function pointer(target:Element,type:string,x:number,button=0){const event=new MouseEvent(type,{bubbles:true,cancelable:true,clientX:x,button});Object.defineProperty(event,'pointerId',{value:1});await fireEvent(target,event);}
function setup(){const callbacks={onview:vi.fn(),onseek:vi.fn(),onselect:vi.fn(),onedit:vi.fn()};const result=render(MapTimeline,{peaks:[[-.2,.2]],duration:12,rate:1000,position:1,view:{start:0,end:8},mapView:timingFixture().view,selected:null,...callbacks});
  const field=screen.getByRole('region',{name:/Measure-map waveform/}),overview=screen.getByRole('button',{name:'Move measure-map view'});
  for(const element of[field,overview]){vi.spyOn(element,'getBoundingClientRect').mockReturnValue({left:100,width:1000,height:200,top:0,right:1100,bottom:200,x:100,y:0,toJSON(){}});const held=new Set<number>();element.setPointerCapture=vi.fn(id=>{held.add(id);});element.hasPointerCapture=vi.fn(id=>held.has(id));element.releasePointerCapture=vi.fn(id=>{held.delete(id);});}
  return{...callbacks,...result,field,overview};
}
describe('measure-map timeline',()=>{
  it('captures a boundary drag and commits one source-frame edit without seeking',async()=>{
    const s=setup();await pointer(screen.getByRole('button',{name:'Measure 2 start'}),'pointerdown',600);await pointer(s.field,'pointermove',650);expect(s.onedit).not.toHaveBeenCalled();
    await pointer(s.field,'pointerup',650);expect(s.onedit).toHaveBeenCalledExactlyOnceWith({type:'move_boundary',id:'map-2',frame:4400});expect(s.onseek).not.toHaveBeenCalled();expect(s.field.releasePointerCapture).toHaveBeenCalledWith(1);
  });
  it('turns an interpolated beat drag into an anchor and supports keyboard adjustment',async()=>{
    const s=setup();const beat=screen.getByRole('button',{name:'Measure 1 beat 2'});await pointer(beat,'pointerdown',225);await pointer(s.field,'pointerup',250);
    expect(s.onedit).toHaveBeenLastCalledWith({type:'set_beat',measure:'map-3',id:null,frame:1200,offset:{num:960,den:960}});
    await fireEvent.keyDown(beat,{key:'ArrowRight',shiftKey:true});expect(s.onedit).toHaveBeenLastCalledWith({type:'set_beat',measure:'map-3',id:null,frame:1001,offset:{num:960,den:960}});
  });
  it('pans and zooms without map edits, cancels pointer gestures, and clamps minimap navigation',async()=>{
    const s=setup();await pointer(s.field,'pointerdown',600,1);await pointer(s.field,'pointermove',700,1);expect(s.onview).toHaveBeenLastCalledWith({start:0,end:8});await pointer(s.field,'pointercancel',700,1);expect(s.onedit).not.toHaveBeenCalled();expect(s.onseek).not.toHaveBeenCalled();
    const wheel=new WheelEvent('wheel',{bubbles:true,cancelable:true,clientX:350,deltaY:-100});await fireEvent(s.field,wheel);expect(wheel.defaultPrevented).toBe(true);expect(s.onview.mock.lastCall?.[0].end-s.onview.mock.lastCall?.[0].start).toBeCloseTo(8*Math.exp(-.25));
    await pointer(s.overview,'pointerdown',900);await pointer(s.overview,'pointermove',2000);await pointer(s.overview,'pointerup',2000);expect(s.onview).toHaveBeenLastCalledWith({start:4,end:12});expect(s.onedit).not.toHaveBeenCalled();
  });
});
