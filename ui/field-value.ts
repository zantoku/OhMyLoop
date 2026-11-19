// Applies a typed number from a field's change event, or restores the shown
// value when the entry is not a number.
export function entered(event: Event & { currentTarget: HTMLInputElement }, shown: number | string, apply: (value: number) => void): void {
  const text = event.currentTarget.value.trim();
  const value = text === '' ? NaN : Number(text);
  if (Number.isFinite(value)) apply(value);
  else event.currentTarget.value = String(shown);
}
