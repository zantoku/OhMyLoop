// Form values and parsing only; musical validation and measure grouping stay in Rust.
import {commonMeter,fractionText,parseFraction,type Meter} from './timing';
export type MeterForm = {numerator:number;denominator:number;pulse:string;grouping:string};
export function meterForm(meter:Meter=commonMeter()):MeterForm {
  return {numerator:meter.numerator,denominator:meter.denominator,pulse:fractionText(meter.pulse),grouping:meter.grouping.join('+')};
}
export function readMeter(form:MeterForm):Meter {
  return {numerator:form.numerator,denominator:form.denominator,pulse:parseFraction(form.pulse),grouping:readGrouping(form.grouping)};
}
export function readGrouping(text:string):number[] {
  const value=text.trim();
  if(!value)return [];
  if(!/^\d+(?:\s*\+\s*\d+)*$/.test(value))throw new Error('Enter grouping as whole numbers separated by +, such as 2+2+3.');
  const grouping=value.split('+').map(v=>Number(v.trim()));
  if(grouping.some(n=>!Number.isSafeInteger(n)))throw new Error('Enter grouping as whole numbers separated by +, such as 2+2+3.');
  return grouping;
}
