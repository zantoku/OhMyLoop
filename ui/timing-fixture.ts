// Small source-frame fixture matching the Rust domain's uniform two-bar map.
import type { MapPreview } from './timing';
export function timingFixture(rate=1000):MapPreview {
  const meter={numerator:4,denominator:4,pulse:{num:1,den:1},grouping:[]};
  const measures=[0,1].map(i=>({id:`map-${i===0?3:5}`,start:`map-${i===0?1:2}`,end:`map-${i===0?2:4}`,meter,kind:'full' as const,length:{num:4,den:1},offset:{num:0,den:1},beats:[],protected:false}));
  return {map:{version:1,revision:1,rate,frames:12*rate,next_id:6,first_number:1,boundaries:[{id:'map-1',frame:0},{id:'map-2',frame:4*rate},{id:'map-4',frame:8*rate}],measures},
    view:{revision:1,measures:measures.map((m,i)=>({id:m.id,start_id:m.start,end_id:m.end,start:i*4*rate,end:(i+1)*4*rate,number:i+1,label:String(i+1),meter,kind:m.kind,length:m.length,protected:false,tempo_bpm:60,split_points:[1,2,3].map(q=>({beat:q+1,frame:(i*4+q)*rate,anchor:false})),grid:[0,1,2,3].map(q=>({id:null,frame:(i*4+q)*rate,quarter:q,label:String(q+1)}))}))}};
}
