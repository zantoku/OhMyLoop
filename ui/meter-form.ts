// Form values and parsing only; musical validation and measure grouping stay in Rust.
import {commonMeter,fractionText,parseFraction,type Meter} from './timing';
export type MeterForm = {numerator:number;denominator:number;pulse:string;grouping:string};
export function meterForm(meter:Meter=commonMeter()):MeterForm {
  return {numerator:meter.numerator,denominator:meter.denominator,pulse:fractionText(meter.pulse),grouping:meter.grouping.join('+')};
}
export function readMeter(form:MeterForm):Meter {
  const grouping=form.grouping.trim()?form.grouping.split('+').map(v=>Number(v.trim())):[];
  if(grouping.some(n=>!Number.isInteger(n)))throw new Error('Enter grouping as whole numbers separated by +, such as 2+2+3.');
  return {numerator:form.numerator,denominator:form.denominator,pulse:parseFraction(form.pulse),grouping};
}
