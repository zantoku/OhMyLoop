import { bridge } from './bridge';
import type { MapEdit, MapPreview, MeasureMap } from './timing';
export type MappingState = { pending: boolean; error: string; preview: MapPreview | null; undo: number; redo: number };
// Edit orchestration and session history, separate from rendering and Rust rules.
export class MappingSession {
  private disposed=false;
  dispose(){this.disposed=true;}
  private past: MeasureMap[] = [];
  private future: MeasureMap[] = [];
  private state: MappingState = { pending: false, error: '', preview: null, undo: 0, redo: 0 };
  constructor(private read: () => { key: string; map: MeasureMap | null }, private commit: (value: MapPreview) => Promise<void>, private publish: (state: MappingState) => void) {}
  private emit() { if(this.disposed)return; this.publish({ ...this.state, undo: this.past.length, redo: this.future.length }); }
  private matches(key: string, revision: number | undefined) { const now = this.read(); return !this.disposed && now.key === key && now.map?.revision === revision; }
  async edit(edit: MapEdit, preview = true) {
    if (this.state.pending) return;
    this.state.pending = true; this.state.error = ''; this.state.preview = null; this.emit();
    const before = this.read();
    try {
      const result = await bridge.previewMap(before.key, before.map, edit);
      if (!this.matches(before.key, before.map?.revision)) throw new Error('The song or map changed. Please try the edit again.');
      if (preview) this.state.preview = result;
      else await this.acceptResult(result, before.map);
    } catch (e) { this.state.error = String(e); }
    finally { this.state.pending = false; this.emit(); }
  }
  private async acceptResult(result: MapPreview, before: MeasureMap | null) {
    await this.commit(result);
    this.past.push(before ?? { ...result.map, revision: 0, next_id: 1, measures: [], boundaries: [] });
    this.past = this.past.slice(-50); this.future = []; this.state.preview = null;
  }
  async apply() {
    if (this.state.pending || !this.state.preview) return;
    const before = this.read(); const result = this.state.preview;
    if (result.map.revision !== (before.map?.revision ?? 0) + 1) { this.state.error = 'The map changed. Rebuild this preview before applying it.'; this.state.preview = null; this.emit(); return; }
    this.state.pending = true; this.emit();
    try { await this.acceptResult(result, before.map); }
    catch (e) { this.state.error = String(e); }
    finally { this.state.pending = false; this.emit(); }
  }
  discard() { if (!this.state.pending) { this.state.preview = null; this.state.error = ''; this.emit(); } }
  async history(direction: 'undo' | 'redo') {
    const from = direction === 'undo' ? this.past : this.future, to = direction === 'undo' ? this.future : this.past;
    if (this.state.pending || !from.length) return;
    const before = this.read(); if (!before.map) return;
    this.state.pending = true; this.state.error = ''; this.state.preview = null; this.emit();
    try {
      const map = structuredClone(from.at(-1)!);
      map.revision = before.map.revision + 1; map.next_id = Math.max(before.map.next_id, map.next_id);
      const view = await bridge.mapView(map);
      if (!this.matches(before.key, before.map.revision)) throw new Error('The song or map changed. Please try again.');
      await this.commit({ map, view }); from.pop(); to.push(before.map);
    } catch (e) { this.state.error = String(e); }
    finally { this.state.pending = false; this.emit(); }
  }
}
