import { expect, it, vi } from 'vitest';
import { fireEvent, render, screen, within } from '@testing-library/svelte';
import SongMapDialog from './SongMapDialog.svelte';
import { Keymap } from './keymap';
import { timingFixture } from './timing-fixture';

const props = (map = timingFixture().map) => ({ open: true, songKey: 'song', map, error: '', keymap: new Keymap(), available: () => true, run: vi.fn(), onclose: vi.fn(), onedit: vi.fn(async () => {}) });

it('lists distinct exact signatures, clears grouping and keeps a chosen signature across revisions', async () => {
  const map = timingFixture().map;
  map.measures[1].meter = { ...map.measures[1].meter, denominator: 8, grouping: [1, 3] };
  const values = props(map);
  const result = render(SongMapDialog, values);
  const dialog = screen.getByRole('dialog', { name: 'Song map settings' });
  expect(within(dialog).getAllByRole('option').map(o => o.textContent)).toEqual(['4/4', '4/8']);
  await fireEvent.change(within(dialog).getByRole('combobox', { name: 'Time signature' }), { target: { value: '4/8' } });
  const input = within(dialog).getByRole('textbox', { name: 'Beat grouping for every measure in this time signature' }) as HTMLInputElement;
  expect(input.value).toBe('1+3');
  await fireEvent.input(input, { target: { value: ' ' } });
  expect(result.component.canRun('map_grouping')).toBe(true);
  await result.component.command('map_grouping');
  expect(values.onedit).toHaveBeenCalledExactlyOnceWith({ type: 'set_grouping_for_meter', numerator: 4, denominator: 8, grouping: [] });
  const revised = structuredClone(map); revised.revision++; revised.measures[1].meter.grouping = [];
  await result.rerender({ ...values, map: revised });
  expect((within(dialog).getByRole('combobox', { name: 'Time signature' }) as HTMLSelectElement).value).toBe('4/8');
  expect(result.component.canRun('map_grouping')).toBe(false);
  await result.rerender({ ...values, songKey: 'other' });
  expect((within(dialog).getByRole('combobox', { name: 'Time signature' }) as HTMLSelectElement).value).toBe('4/4');
});

it('reports locked matches and disables grouping with no changes', async () => {
  const map = timingFixture().map;
  for (const measure of map.measures) measure.protected = true;
  const result = render(SongMapDialog, { ...props(map), available: () => false });
  await fireEvent.input(screen.getByRole('textbox', { name: 'Beat grouping for every measure in this time signature' }), { target: { value: '2+2' } });
  expect(screen.getByText(/^0 measures change · 2 locked measures skipped\./)).toBeTruthy();
  expect(result.component.canRun('map_grouping')).toBe(false);
  for (const button of screen.getAllByRole('button', { name: 'Preview' })) expect(button.hasAttribute('disabled')).toBe(true);
});

it('previews numbering, shows a rejection, and closes with its X or Esc', async () => {
  const values = props();
  const result = render(SongMapDialog, { ...values, error: 'Error: Choose a number from 1' });
  const dialog = screen.getByRole('dialog', { name: 'Song map settings' });
  await fireEvent.input(within(dialog).getByRole('spinbutton', { name: 'First full measure is number' }), { target: { value: '12' } });
  await result.component.command('map_origin');
  expect(values.onedit).toHaveBeenCalledExactlyOnceWith({ type: 'set_origin', first_number: 12 });
  expect(within(dialog).getByRole('alert').textContent).toBe('Error: Choose a number from 1');
  await fireEvent.click(within(dialog).getByRole('button', { name: 'Close song map settings' }));
  await fireEvent.keyDown(dialog, { key: 'Escape' });
  expect(values.onclose).toHaveBeenCalledTimes(2);
  // Previewing submits the form, so Enter in a field previews too.
  await fireEvent.submit(within(dialog).getByRole('spinbutton', { name: 'First full measure is number' }).closest('form')!);
  expect(values.run).toHaveBeenCalledWith('map_origin');
});
