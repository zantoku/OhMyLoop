// Open transient UI (popovers, menus, inline confirmations), newest last. Esc
// unwinds one level per press, so the key handler that owns Esc closes the
// newest one before anything else.
import { getContext, setContext } from 'svelte';

export class TransientStack {
  private open: (() => void)[] = [];
  /** Registers open transient UI; call the returned function when it closes. */
  add(close: () => void): () => void {
    this.open.push(close);
    return () => { const index = this.open.lastIndexOf(close); if (index >= 0) this.open.splice(index, 1); };
  }
  /** Closes the newest open transient UI; false when none is open. */
  closeTop(): boolean {
    const close = this.open.pop();
    close?.();
    return !!close;
  }
}

// The window's stack, unwound by the global key handler.
const windowStack = new TransientStack();

/** Registers an open popover; call the returned function when it closes. */
export function openTransient(close: () => void): () => void { return windowStack.add(close); }

/** Closes the newest open transient UI; false when none is open. */
export function closeTopTransient(): boolean { return windowStack.closeTop(); }

const CONTEXT = Symbol('transient-stack');
/**
 * A modal dialog owns its keys, so transient UI inside it unwinds through the
 * dialog's own stack. Call during the dialog component's initialisation.
 */
export function provideTransientStack(stack: TransientStack): void { setContext(CONTEXT, stack); }
/** The register function for the enclosing dialog's stack, or the window's. Call during initialisation. */
export function transientRegistrar(): (close: () => void) => () => void {
  const stack = getContext<TransientStack | undefined>(CONTEXT);
  return stack ? close => stack.add(close) : openTransient;
}
