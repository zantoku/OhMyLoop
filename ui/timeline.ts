import { clamp, zoom } from './model';
import type { View } from './types';
export function sourcePoint(view: View, duration: number, box: Pick<DOMRect, 'left' | 'width'>, x: number): number {
  return clamp(view.start + (x - box.left) / Math.max(1, box.width) * (view.end - view.start), 0, duration);
}
export function panView(original: View, duration: number, deltaPixels: number, width: number): View {
  const span = original.end - original.start;
  const start = clamp(original.start - deltaPixels / Math.max(1, width) * span, 0, duration - span);
  return { start, end: start + span };
}
export function overviewView(view: View, duration: number, box: Pick<DOMRect, 'left' | 'width'>, x: number): View {
  const span = view.end - view.start, center = (x - box.left) / Math.max(1, box.width) * duration;
  const start = clamp(center - span / 2, 0, duration - span);
  return { start, end: start + span };
}
export function wheelView(view: View, duration: number, box: DOMRect, event: WheelEvent): View {
  const unit = event.deltaMode === 1 ? 16 : event.deltaMode === 2 ? box.height : 1;
  const factor = Math.exp(clamp(event.deltaY * unit, -240, 240) * .0025);
  return zoom(view, duration, factor, sourcePoint(view, duration, box, event.clientX), clamp((event.clientX-box.left) / Math.max(1, box.width), 0, 1));
}
export function drawWaveform(target: HTMLCanvasElement | undefined, peaks: [number, number][], duration: number, start: number, end: number, overview = false) {
  if (!target || !peaks.length) return;
  const ctx = target.getContext('2d'); if (!ctx) return;
  const box = target.getBoundingClientRect(), dpr = window.devicePixelRatio || 1;
  target.width = Math.round(box.width*dpr); target.height = Math.round(box.height*dpr);
  ctx.scale(dpr,dpr); const w=box.width,h=box.height; ctx.clearRect(0,0,w,h);
  const max=Math.max(.01,...peaks.filter((_,i)=>i%16===0).map(p=>Math.max(-p[0],p[1]))), scale=h*.39/max;
  ctx.strokeStyle='rgba(209,221,245,.12)'; ctx.lineWidth=1; ctx.beginPath(); ctx.moveTo(0,h/2); ctx.lineTo(w,h/2); ctx.stroke();
  ctx.fillStyle=overview?'#adbcdb':'#bccceb';
  for(let x=0;x<w;x+=2){
    const first=Math.floor((start+(end-start)*x/w)/duration*peaks.length);
    const last=Math.min(peaks.length,Math.max(first+1,Math.ceil((start+(end-start)*(x+2)/w)/duration*peaks.length)));
    let low=0,high=0;for(let i=first;i<last;i++){const p=peaks[i];if(p){low=Math.min(low,p[0]);high=Math.max(high,p[1]);}}
    ctx.fillRect(x,h/2-high*scale,1.4,Math.max(1,(high-low)*scale));
  }
}
