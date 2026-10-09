import {describe,expect,it} from 'vitest';
import {pageForPlayhead} from './timeline';
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
