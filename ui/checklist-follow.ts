/** Follow only the checklist viewport; a user's scroll away suspends following. */
export type ChecklistGeometry = {top:number;bottom:number;rowTop:number;rowBottom:number};
// Browser scrollTop may round while row bounds retain fractional CSS pixels.
export const activeRowVisible=({top,bottom,rowTop,rowBottom}:ChecklistGeometry)=>rowTop>=top-1&&rowBottom<=bottom+1;
export class ChecklistFollow {
 private following=true;
 reset(){this.following=true;}
 scrolled(geometry:ChecklistGeometry){this.following=activeRowVisible(geometry);}
 reveal(geometry:ChecklistGeometry):number {
  if(!this.following||activeRowVisible(geometry))return 0;
  return geometry.rowTop<geometry.top?geometry.rowTop-geometry.top:geometry.rowBottom-geometry.bottom;
 }
}
