use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Fraction {
    pub num: u32,
    pub den: u32,
}
impl Fraction {
    pub fn new(num: u32, den: u32) -> Self {
        let mut a = num;
        let mut b = den;
        while b != 0 {
            (a, b) = (b, a % b);
        }
        let divisor = a.max(1);
        Self {
            num: num / divisor,
            den: den / divisor,
        }
    }
    pub fn value(self) -> f64 {
        self.num as f64 / self.den as f64
    }
    pub(super) fn validate(self, positive: bool) -> Result<()> {
        ensure!(
            self.den > 0 && self.den <= 960 && self.num <= 65_536 && (!positive || self.num > 0),
            "Invalid notated duration"
        );
        Ok(())
    }
    pub(super) fn add(self, other: Self) -> Self {
        Self::new(
            self.num * other.den + other.num * self.den,
            self.den * other.den,
        )
    }
    pub(super) fn sub(self, other: Self) -> Result<Self> {
        ensure!(
            self.value() >= other.value(),
            "Notated position is outside the measure"
        );
        Ok(Self::new(
            self.num * other.den - other.num * self.den,
            self.den * other.den,
        ))
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Meter {
    pub numerator: u32,
    pub denominator: u32,
    pub pulse: Fraction,
    pub grouping: Vec<u32>,
}
impl Meter {
    pub fn common() -> Self {
        Self {
            numerator: 4,
            denominator: 4,
            pulse: Fraction::new(1, 1),
            grouping: vec![],
        }
    }
    pub fn length(&self) -> Fraction {
        Fraction::new(self.numerator * 4, self.denominator)
    }
    pub(super) fn validate(&self) -> Result<()> {
        ensure!(
            (1..=99).contains(&self.numerator)
                && [1, 2, 4, 8, 16, 32, 64].contains(&self.denominator),
            "Choose a valid time signature"
        );
        self.pulse.validate(true)?;
        ensure!(
            self.pulse.value() <= self.length().value(),
            "Pulse is longer than the measure"
        );
        ensure!(
            self.grouping.len() <= 99
                && (self.grouping.is_empty()
                    || (self.grouping.iter().all(|v| *v > 0 && *v <= 99)
                        && self.grouping.iter().sum::<u32>() == self.numerator)),
            "Grouping must add up to the time-signature numerator"
        );
        Ok(())
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Boundary {
    pub id: String,
    pub frame: u64,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Beat {
    pub id: String,
    pub frame: u64,
    pub offset: Fraction,
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MeasureKind {
    Full,
    Pickup,
    Incomplete,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Measure {
    pub id: String,
    pub start: String,
    pub end: String,
    pub meter: Meter,
    pub kind: MeasureKind,
    pub length: Fraction,
    pub offset: Fraction,
    pub beats: Vec<Beat>,
    pub protected: bool,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct MeasureMap {
    pub version: u32,
    pub revision: u64,
    pub rate: u32,
    pub frames: u64,
    pub next_id: u64,
    pub first_number: u32,
    pub boundaries: Vec<Boundary>,
    pub measures: Vec<Measure>,
}
impl MeasureMap {
    pub fn empty(rate: u32, frames: u64) -> Self {
        Self {
            version: 1,
            revision: 0,
            rate,
            frames,
            next_id: 1,
            first_number: 1,
            boundaries: vec![],
            measures: vec![],
        }
    }
    pub(super) fn id(&mut self) -> String {
        let id = format!("map-{}", self.next_id);
        self.next_id += 1;
        id
    }
    pub(super) fn boundary(&mut self, frame: u64) -> String {
        if let Some(b) = self.boundaries.iter().find(|b| b.frame == frame) {
            return b.id.clone();
        }
        let id = self.id();
        self.boundaries.push(Boundary {
            id: id.clone(),
            frame,
        });
        id
    }
}
