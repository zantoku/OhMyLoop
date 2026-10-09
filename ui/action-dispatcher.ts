import { ACTIONS, type ActionId } from './actions';
import type { Section } from './types';

export type ActionPayload = { section?: Section; speed?: number };
export type ActionHandlers = Record<ActionId, (payload: ActionPayload) => void | Promise<void>>;
export type ActionState = {
  context?: 'practice' | 'mapping'; busy: boolean; song: boolean; range: boolean; sections: boolean; audio: boolean;
  loop_on: boolean; undo: boolean; editor: boolean; help: boolean; zoom_out: boolean;
};

// Keyboard and buttons share availability and execution, including contextual payloads.
export class ActionDispatcher {
  constructor(private handlers: ActionHandlers, private state: () => ActionState) {}

  available(id: ActionId): boolean {
    const state = this.state();
    const scope = ACTIONS[id].scope;
    if (scope !== 'all' && scope !== (state.context ?? 'practice')) return false;
    if (state.busy && !ACTIONS[id].duringBusy) return false;
    const requirement = ACTIONS[id].requires;
    return requirement === 'none' || state[requirement];
  }

  run(id: ActionId, payload: ActionPayload = {}): void | Promise<void> {
    if (!this.available(id)) return;
    if (ACTIONS[id].payload === 'section' && !payload.section) return;
    if (ACTIONS[id].payload === 'speed' && !Number.isFinite(payload.speed)) return;
    return this.handlers[id](payload);
  }
}
