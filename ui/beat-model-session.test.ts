import { beforeEach, expect, it, vi } from 'vitest';
import { BeatModelSession, type BeatModelState } from './beat-model-session';
import type { AnalysisStatus } from './timing';

const idle: AnalysisStatus = { id: 1, running: false, kind: 'idle', phase: 'Ready', model_ready: false, model_path: '/models', progress: null, elapsed_seconds: 0, reference_token: null, map_revision: 0, error: null, result: null };
const api = { analysisStatus: vi.fn(), installModel: vi.fn(), removeModel: vi.fn(), cancelAnalysis: vi.fn() };
let state: BeatModelState; let session: BeatModelSession;
beforeEach(() => { vi.resetAllMocks(); session = new BeatModelSession(api, value => { state = value; }); });

it('installs, cancels only a running download, and removes the model', async () => {
  const downloading = { ...idle, id: 2, running: true, kind: 'install', phase: 'Downloading beat model' };
  api.installModel.mockResolvedValue(downloading); await session.install();
  expect(state.status?.running).toBe(true); expect(state.pending).toBe(false);
  api.cancelAnalysis.mockResolvedValue({ ...downloading, phase: 'Cancelling' }); await session.cancel();
  expect(api.cancelAnalysis).toHaveBeenCalledWith(2);
  api.removeModel.mockResolvedValue({ ...idle, model_ready: false }); await session.remove();
  expect(api.removeModel).toHaveBeenCalledOnce(); expect(state.status?.model_ready).toBe(false);
});

it('never cancels an analysis run, and reports a failed download without dropping the status', async () => {
  api.analysisStatus.mockResolvedValue({ ...idle, running: true, kind: 'analysis' }); await session.refresh();
  await session.cancel(); expect(api.cancelAnalysis).not.toHaveBeenCalled();
  api.installModel.mockRejectedValue(new Error('Network unavailable')); await session.install();
  expect(state.error).toContain('Network unavailable'); expect(state.status?.kind).toBe('analysis');
});

it('ignores a poll reply that started before a command', async () => {
  let finish!: (value: AnalysisStatus) => void; api.analysisStatus.mockReturnValue(new Promise(done => { finish = done; }));
  const poll = session.refresh();
  api.installModel.mockResolvedValue({ ...idle, id: 3, running: true, kind: 'install' }); await session.install();
  finish({ ...idle, id: 1 }); await poll;
  expect(state.status?.id).toBe(3);
});
