// The beat analysis model's installation, owned by App so Settings and the
// Measure map see the same download. Analysis runs themselves stay in
// AnalysisSession. Replies to an older command never replace a newer one.
import type { AnalysisStatus } from './timing';

type API = {
  analysisStatus: () => Promise<AnalysisStatus>; installModel: () => Promise<AnalysisStatus>;
  removeModel: () => Promise<AnalysisStatus>; cancelAnalysis: (id: number) => Promise<AnalysisStatus>;
};
export type BeatModelState = { status: AnalysisStatus | null; pending: boolean; error: string };

export class BeatModelSession {
  private state: BeatModelState = { status: null, pending: false, error: '' };
  private epoch = 0;
  private polling = false;
  private disposed = false;
  constructor(private api: API, private publish: (state: BeatModelState) => void) {}
  private emit() { if (!this.disposed) this.publish({ ...this.state }); }

  async refresh() {
    if (this.polling || this.disposed) return;
    this.polling = true; const epoch = this.epoch;
    try {
      const status = await this.api.analysisStatus();
      if (epoch === this.epoch && !this.disposed) { this.state.status = status; this.emit(); }
    } catch (e) {
      if (epoch === this.epoch && !this.disposed) { this.state.error = String(e); this.emit(); }
    } finally { this.polling = false; }
  }

  private async command(work: () => Promise<AnalysisStatus>) {
    if (this.state.pending || this.disposed) return;
    const epoch = ++this.epoch; this.state.pending = true; this.state.error = ''; this.emit();
    try { const status = await work(); if (epoch === this.epoch) this.state.status = status; }
    catch (e) { if (epoch === this.epoch) this.state.error = String(e); }
    finally { if (epoch === this.epoch) { this.state.pending = false; this.emit(); } }
  }
  install() { return this.command(this.api.installModel); }
  remove() { return this.command(this.api.removeModel); }
  /** Cancels the running download; an analysis run is cancelled from the Measure map. */
  cancel() {
    const status = this.state.status;
    if (status?.running && status.kind === 'install') return this.command(() => this.api.cancelAnalysis(status.id));
  }
  dispose() { this.disposed = true; this.epoch++; }
}
