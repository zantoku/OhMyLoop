import { describe, expect, it, vi } from 'vitest';
import { dropPreview, dropRefusal, FileDropTarget } from './file-drop';

describe('dropping files onto the window', () => {
  it('accepts exactly one file and names it', () => {
    expect(dropRefusal(['/music/Spain.flac'])).toBeNull();
    expect(dropPreview(['/music/Spain.flac'])).toEqual({ name: 'Spain.flac' });
    expect(dropPreview(['C:\\Music\\Spain.flac'])).toEqual({ name: 'Spain.flac' });
    expect(dropRefusal(['/a.wav', '/b.wav', '/c.wav'])).toMatch(/^Drop one audio file at a time\. 3 files were dropped/);
    expect(dropPreview(['/a.wav', '/b.wav'])).toEqual({ refused: 'Drop one audio file at a time' });
  });

  it('shows the drop target while dragging, then imports the one file', () => {
    const show = vi.fn(), importFile = vi.fn(), refuse = vi.fn();
    const target = new FileDropTarget(() => true, show, importFile, refuse);
    target.handle({ type: 'enter', paths: ['/music/Spain.flac'] });
    expect(show).toHaveBeenLastCalledWith({ name: 'Spain.flac' });
    target.handle({ type: 'over' });
    target.handle({ type: 'drop', paths: ['/music/Spain.flac'] });
    expect(show).toHaveBeenLastCalledWith(null);
    expect(importFile).toHaveBeenCalledExactlyOnceWith('/music/Spain.flac');
    expect(refuse).not.toHaveBeenCalled();
  });

  it('refuses several files with an explanation and imports nothing', () => {
    const importFile = vi.fn(), refuse = vi.fn();
    const target = new FileDropTarget(() => true, () => {}, importFile, refuse);
    target.handle({ type: 'enter', paths: ['/a.wav', '/b.wav'] });
    target.handle({ type: 'drop', paths: ['/a.wav', '/b.wav'] });
    expect(refuse).toHaveBeenCalledOnce();
    expect(importFile).not.toHaveBeenCalled();
  });

  it('ignores a drag while an import can’t start, and a drag that left', () => {
    let accepts = false;
    const show = vi.fn(), importFile = vi.fn(), refuse = vi.fn();
    const target = new FileDropTarget(() => accepts, show, importFile, refuse);
    target.handle({ type: 'enter', paths: ['/a.wav'] });
    expect(show).toHaveBeenLastCalledWith(null);
    target.handle({ type: 'drop', paths: ['/a.wav'] });
    accepts = true;
    target.handle({ type: 'enter', paths: ['/a.wav'] });
    target.handle({ type: 'leave' });
    target.handle({ type: 'drop', paths: ['/a.wav'] });
    expect(importFile).not.toHaveBeenCalled();
    expect(refuse).not.toHaveBeenCalled();
  });
});
