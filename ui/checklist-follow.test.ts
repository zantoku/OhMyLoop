import {describe,it,expect} from 'vitest';
import {ChecklistFollow} from './checklist-follow';
const visible={top:10,bottom:110,rowTop:30,rowBottom:70};
const below={...visible,rowTop:120,rowBottom:160};
describe('checklist following',()=>{
 it('reveals an active row and leaves visible rows still',()=>{const follow=new ChecklistFollow();expect(follow.reveal(visible)).toBe(0);expect(follow.reveal(below)).toBe(50);expect(follow.reveal({...visible,rowTop:-20,rowBottom:0})).toBe(-30);});
 it('pauses when scrolling away, resumes at the active row, and resets on reopening or a new attempt',()=>{const follow=new ChecklistFollow();follow.scrolled(below);expect(follow.reveal(below)).toBe(0);follow.scrolled(visible);expect(follow.reveal(below)).toBe(50);follow.scrolled(below);follow.reset();expect(follow.reveal(below)).toBe(50);});
 it('allows subpixel scroll rounding without suspending following',()=>{const follow=new ChecklistFollow();follow.scrolled({...visible,rowTop:9.7,rowBottom:110.3});expect(follow.reveal(below)).toBe(50);});
 it('does not confuse an immediate programmatic reveal with user override',()=>{const follow=new ChecklistFollow();expect(follow.reveal(below)).toBe(50);follow.scrolled({...below,rowTop:70,rowBottom:110});expect(follow.reveal(below)).toBe(50);});
});
