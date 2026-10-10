import type {MeasureView} from './timing';
export type UnmappedRange = {start:number;end:number};
// Presentation ranges between Rust-projected measures, in source frames.
export function unmappedRanges(measures:readonly MeasureView[],frames:number):UnmappedRange[]{
  const ranges:UnmappedRange[]=[];let start=0;
  for(const m of measures){if(m.start>start)ranges.push({start,end:m.start});start=m.end;}
  if(start<frames)ranges.push({start,end:frames});return ranges;
}
