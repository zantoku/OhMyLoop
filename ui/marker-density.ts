// Presentation only: retain a stable marker phase while panning. Audio timing
// and the full editable map remain independent of the visible detail level.
// Input is ordered by source frame, as supplied by the timing/analysis DTOs.
export function markersForView<T extends {frame:number}>(points:readonly T[],start:number,end:number,width:number,minPixels=24):{markers:T[];stride:number} {
  if(end<=start||width<=0)return {markers:[],stride:1};
  const bound=(frame:number,inclusive:boolean)=>{
    let low=0,high=points.length;
    while(low<high){const mid=Math.floor((low+high)/2);if(inclusive?points[mid].frame<=frame:points[mid].frame<frame)low=mid+1;else high=mid;}return low;
  };
  const first=bound(start,false),last=bound(end,true);
  const visible=points.slice(first,last).map((point,index)=>({point,index:first+index}));
  const gaps=visible.slice(1).map((p,i)=>p.point.frame-visible[i].point.frame).filter(gap=>gap>0).sort((a,b)=>a-b);
  // A lower quartile reflects crowded passages without letting a single close
  // anchor or a free-time gap set the detail level for the entire viewport.
  const spacing=gaps.length?gaps[Math.floor((gaps.length-1)/4)]/(end-start)*width:visible.length>1?0:Infinity;
  const needed=Math.max(1,minPixels/spacing);let stride=1;
  if(!Number.isFinite(needed))return {markers:visible.slice(0,1).map(({point})=>point),stride:Math.max(1,points.length)};
  for(let scale=1;scale<=points.length*10;scale*=10){
    const step=[1,2,4,10].map(n=>n*scale).find(n=>n>=needed);
    if(step){stride=step;break;}
  }
  return {markers:visible.filter(({index})=>index%stride===0).map(({point})=>point),stride};
}
