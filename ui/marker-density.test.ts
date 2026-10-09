import {describe,expect,it} from 'vitest';
import {markersForView} from './marker-density';
const points=Array.from({length:101},(_,i)=>({frame:i*100,id:String(i)}));
describe('waveform marker detail',()=>{
  it.each([[2400,1],[1200,2],[600,4],[240,10],[120,20]])('uses a stable every-nth step at width %s', (width,stride)=>{
    const result=markersForView(points,0,10_000,width);
    expect(result.stride).toBe(stride);expect(result.markers.every(p=>Number(p.id)%stride===0)).toBe(true);
    expect(result.markers).toEqual(points.filter((_,i)=>i%stride===0));
  });
  it('keeps the same phase when panning and restores full detail on zoom-in',()=>{
    const left=markersForView(points,0,5000,300),right=markersForView(points,100,5100,300);
    expect(left.stride).toBe(4);expect(right.stride).toBe(4);
    expect(left.markers.filter(p=>p.frame>=100)).toEqual(right.markers.filter(p=>p.frame<=5000));
    expect(markersForView(points,0,500,300).markers).toEqual(points.slice(0,6));
    expect(points).toHaveLength(101);
  });
  it('handles coincident rounded anchors, gaps and an empty/zero-size viewport',()=>{
    expect(markersForView([{frame:1},{frame:1},{frame:1}],0,100,200).markers).toHaveLength(1);
    expect(markersForView([],0,100,200)).toEqual({markers:[],stride:1});
    expect(markersForView(points,0,100,0).markers).toEqual([]);
    const sparseTail=[...points.slice(0,20),{frame:10_000,id:'tail'}];
    expect(markersForView(sparseTail,0,10_000,240).stride).toBe(10);
  });
});
