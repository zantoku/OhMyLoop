import {describe,expect,it} from 'vitest';
import {unmappedRanges} from './map-ranges';
import {timingFixture} from './timing-fixture';
describe('uncounted source ranges',()=>{
  it('includes leading, internal and trailing gaps without altering projected measures',()=>{
    const measures=timingFixture().view.measures;measures[0].start=1000;measures[0].end=3000;measures[1].start=5000;measures[1].end=7000;
    const before=structuredClone(measures);
    expect(unmappedRanges(measures,10000)).toEqual([{start:0,end:1000},{start:3000,end:5000},{start:7000,end:10000}]);
    expect(measures).toEqual(before);
  });
  it('offers the whole source for an empty map and no range for a complete map',()=>{
    expect(unmappedRanges([],12000)).toEqual([{start:0,end:12000}]);
    expect(unmappedRanges(timingFixture().view.measures,8000)).toEqual([]);
  });
});
