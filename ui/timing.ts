// DTOs from Rust's musical-time domain. This module contains no map editing rules.
export type Fraction = { num: number; den: number };
export type Meter = { numerator: number; denominator: number; pulse: Fraction; grouping: number[] };
export type MeasureKind = 'full' | 'pickup' | 'incomplete';
export type Beat = { id: string; frame: number; offset: Fraction };
export type Measure = { id: string; start: string; end: string; meter: Meter; kind: MeasureKind; length: Fraction; offset: Fraction; beats: Beat[]; protected: boolean };
export type MeasureMap = { version: number; revision: number; rate: number; frames: number; next_id: number; first_number: number; boundaries: { id: string; frame: number }[]; measures: Measure[] };
export type GridPoint = { id: string | null; frame: number; quarter: number; label: string };
export type MeasureView = { id: string; start_id: string; end_id: string; start: number; end: number; number: number | null; label: string; meter: Meter; kind: MeasureKind; length: Fraction; protected: boolean; grid: GridPoint[] };
export type MapView = { revision: number; measures: MeasureView[] };
export type MapPreview = { map: MeasureMap; view: MapView };
export type MusicalPosition = { measure: string; label: string; beat: number };
export type MapEdit =
  | { type: 'build_from_pulses'; pulses:number[]; start:number; end:number; meter:Meter }
  | { type: 'build'; start: number; end: number; count: number; meter: Meter }
  | { type: 'move_boundary'; id: string; frame: number }
  | { type: 'set_notation'; id: string; meter: Meter; kind: MeasureKind; length: Fraction; following: boolean }
  | { type: 'set_protected'; id: string; protected: boolean }
  | { type: 'remove'; id: string }
  | { type: 'set_beat'; measure: string; id: string | null; frame: number; offset: Fraction }
  | { type: 'remove_beat'; measure: string; id: string }
  | { type: 'split'; id: string; frame: number; left: Fraction }
  | { type: 'merge'; id: string; meter: Meter }
  | { type: 'respace'; first: string; last: string };
export const commonMeter = (): Meter => ({ numerator: 4, denominator: 4, pulse: { num: 1, den: 1 }, grouping: [] });
export const fractionText = (f: Fraction): string => f.den === 1 ? String(f.num) : `${f.num}/${f.den}`;
// Form parsing only. Rust validates all resulting values and musical semantics.
export function parseFraction(text: string): Fraction {
  const match = /^(\d+)(?:\/(\d+))?$/.exec(text.trim());
  if (!match) throw new Error('Use a whole number or fraction, such as 1 or 3/2.');
  const num = Number(match[1]), den = Number(match[2] ?? 1);
  if (!Number.isSafeInteger(num) || !Number.isSafeInteger(den) || den < 1 || num > 65536 || den > 960) throw new Error('Choose a smaller valid note value.');
  return { num, den };
}
export type Detection = { frame:number; score:number; downbeat:boolean };
export type Candidates = { version:number; song_key:string; map_revision:number; rate:number; frames:number; model:string; preprocessing:string; elapsed_seconds:number; beats:Detection[] };
export type AnalysisStatus = { id:number; running:boolean; kind:string; phase:string; model_ready:boolean; song_key:string|null; map_revision:number; error:string|null; result:Candidates|null };
