import {describe,it,expect,vi} from 'vitest';
import {render,screen,fireEvent} from '@testing-library/svelte';
import TrackMixer from './TrackMixer.svelte';
import {Keymap} from './keymap';
const tracks=[{role:'bass' as const,gain:.7,mute:false,solo:false}];
describe('accessible rotary mixer controls',()=>{
 it('adjusts gain with arrows and wheel, resets by double-click, and keeps mute/solo independent',async()=>{
 const ongain=vi.fn(),onmute=vi.fn(),onsolo=vi.fn();render(TrackMixer,{tracks,keymap:new Keymap(),ongain,onmute,onsolo});const knob=screen.getByRole('slider',{name:'Bass gain'});
 await fireEvent.keyDown(knob,{key:'ArrowUp'});expect(ongain).toHaveBeenLastCalledWith('bass',.71);
 await fireEvent.wheel(knob,{deltaY:10});expect(ongain).toHaveBeenLastCalledWith('bass',.68);
 await fireEvent.doubleClick(knob);expect(ongain).toHaveBeenLastCalledWith('bass',1);
 await fireEvent.click(screen.getByRole('button',{name:'Mute Bass'}));expect(onmute).toHaveBeenCalledWith('bass');await fireEvent.click(screen.getByRole('button',{name:'Solo Bass'}));expect(onsolo).toHaveBeenCalledWith('bass');
 expect(screen.getByRole('button',{name:'Mute Bass'}).title).toContain('(1)');expect(knob.getAttribute('aria-valuenow')).toBe('70');
 });
 it('keeps unavailable controls visible and blocks all input modalities',async()=>{
 const ongain=vi.fn(),onmute=vi.fn(),onsolo=vi.fn();render(TrackMixer,{tracks,keymap:new Keymap(),disabled:true,ongain,onmute,onsolo});const knob=screen.getByRole('slider',{name:'Bass gain'});
 await fireEvent.keyDown(knob,{key:'End'});await fireEvent.wheel(knob,{deltaY:-1});await fireEvent.doubleClick(knob);expect(ongain).not.toHaveBeenCalled();expect(knob.tabIndex).toBe(-1);expect(screen.getByRole('button',{name:'Mute Bass'}).hasAttribute('disabled')).toBe(true);
 });
});
