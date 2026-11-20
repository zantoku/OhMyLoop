import {describe,expect,it} from 'vitest';
import {fitRange,followPage,pageForPlayhead,revealRange} from './timeline';
describe('playhead paging',()=>{
  const view={start:2,end:5};
  it('holds the current page until its exclusive end',()=>{
    expect(pageForPlayhead(view,12,4.999)).toBe(view);
    expect(pageForPlayhead(view,12,5)).toEqual({start:5,end:8});
  });
  it('handles skipped pages, backwards seeks and loop wraps',()=>{
    expect(pageForPlayhead(view,12,8.5)).toEqual({start:8,end:11});
    expect(pageForPlayhead({start:8,end:11},12,2.5)).toEqual({start:2,end:5});
    expect(pageForPlayhead(view,12,0)).toEqual({start:0,end:3});
  });
  it('preserves zoom at the final page and stops at EOF',()=>{
    const last=pageForPlayhead(view,12,11);
    expect(last).toEqual({start:9,end:12});
    expect(pageForPlayhead(last,12,12)).toBe(last);
    expect(pageForPlayhead({start:0,end:12},12,12)).toEqual({start:0,end:12});
  });
});
describe('following and revealing',()=>{
  const view={start:0,end:4};
  it('pages on start, at the right edge and on wraps, and holds a manual pan otherwise',()=>{
    const paused={position:5,playing:false};
    expect(followPage(view,12,paused,paused)).toBe(view);
    expect(followPage(view,12,paused,{position:5,playing:true})).toEqual({start:4,end:8});
    expect(followPage(view,12,{position:3.9,playing:true},{position:4,playing:true})).toEqual({start:4,end:8});
    expect(followPage({start:4,end:8},12,{position:6,playing:true},{position:1,playing:true})).toEqual({start:0,end:4});
    // A pan away from the playhead holds while it plays on inside the old page.
    const panned={start:8,end:12};
    expect(followPage(panned,12,{position:2,playing:true},{position:2.1,playing:true})).toBe(panned);
  });
  it('fits a range with room around it, within the song',()=>{
    expect(fitRange(12,4,8)).toEqual({start:3.4,end:8.6});
    expect(fitRange(12,0,12)).toEqual({start:0,end:12});
  });
  it('reveals a range by centring it, keeping zoom when it fits',()=>{
    expect(revealRange(view,12,1,2)).toBe(view);
    expect(revealRange(view,12,8,9)).toEqual({start:6.5,end:10.5});
    expect(revealRange(view,12,2,10)).toEqual({start:2,end:10});
  });
});
