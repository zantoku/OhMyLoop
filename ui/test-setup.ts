import { afterEach, vi } from 'vitest';
import { cleanup } from '@testing-library/svelte';

class ResizeObserverStub { observe() {} unobserve() {} disconnect() {} }
vi.stubGlobal('ResizeObserver', ResizeObserverStub);
vi.spyOn(HTMLCanvasElement.prototype, 'getContext').mockReturnValue(null);
HTMLDialogElement.prototype.showModal = function () { this.setAttribute('open', ''); };
HTMLDialogElement.prototype.close = function () { this.removeAttribute('open'); this.dispatchEvent(new Event('close')); };
afterEach(() => cleanup());
