import {expect,it,vi} from 'vitest';
import {fireEvent,render,screen} from '@testing-library/svelte';
import BatchTools from './BatchTools.svelte';
import {Keymap} from './keymap';
import {timingFixture} from './timing-fixture';

it('lists distinct exact signatures, clears grouping and preserves a chosen signature across revisions',async()=>{
  const map=timingFixture().map;
  map.measures[1].meter={...map.measures[1].meter,denominator:8,grouping:[1,3]};
  const onedit=vi.fn();
  const props={songKey:'song',map,editable:true,keymap:new Keymap(),available:()=>true,onedit,run:vi.fn()};
  const result=render(BatchTools,props);
  expect(screen.getAllByRole('option').map(o=>o.textContent)).toEqual(['4/4','4/8']);
  await fireEvent.change(screen.getByRole('combobox',{name:'Time signature'}),{target:{value:'4/8'}});
  const input=screen.getByRole('textbox',{name:'Batch beat grouping (optional)'}) as HTMLInputElement;
  expect(input.value).toBe('1+3');
  await fireEvent.input(input,{target:{value:' '}});
  expect(result.component.canRun('map_grouping')).toBe(true);
  await result.component.command('map_grouping');
  expect(onedit).toHaveBeenCalledExactlyOnceWith({type:'set_grouping_for_meter',numerator:4,denominator:8,grouping:[]});
  const revised=structuredClone(map);revised.revision++;revised.measures[1].meter.grouping=[];
  await result.rerender({...props,map:revised});
  expect((screen.getByRole('combobox',{name:'Time signature'}) as HTMLSelectElement).value).toBe('4/8');
  expect(result.component.canRun('map_grouping')).toBe(false);
  await result.rerender({...props,songKey:'other'});
  expect((screen.getByRole('combobox',{name:'Time signature'}) as HTMLSelectElement).value).toBe('4/4');
});

it('reports locked matches and disables grouping with no changes or no map',async()=>{
  const map=timingFixture().map;
  for(const measure of map.measures)measure.protected=true;
  const props={songKey:'song',map,editable:true,keymap:new Keymap(),available:()=>false,onedit:vi.fn(),run:vi.fn()};
  const result=render(BatchTools,props);
  await fireEvent.input(screen.getByRole('textbox',{name:'Batch beat grouping (optional)'}),{target:{value:'2+2'}});
  expect(screen.getByText('0 measures will change; 2 locked measures skipped.')).toBeTruthy();
  expect(result.component.canRun('map_grouping')).toBe(false);
  expect(screen.getByRole('button',{name:'Preview grouping'}).hasAttribute('disabled')).toBe(true);
  await result.rerender({...props,map:null});
  expect(result.component.canRun('map_grouping')).toBe(false);
  expect((screen.getByRole('combobox',{name:'Time signature'}) as HTMLSelectElement).disabled).toBe(true);
  expect((screen.getByRole('textbox',{name:'Batch beat grouping (optional)'}) as HTMLInputElement).disabled).toBe(true);
});
