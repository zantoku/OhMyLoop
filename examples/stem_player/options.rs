// SPDX-License-Identifier: GPL-3.0-or-later
use anyhow::{Context, Result, ensure};
use ohmyloop::timing::{Edit, Fraction, MeasureMap, Meter};
use std::path::PathBuf;

pub struct Options {
    pub reference: PathBuf,
    pub sets: Vec<PathBuf>,
    pub bpm: Option<f64>,
    pub meter: Meter,
    pub check: bool,
    pub smoke: bool,
    pub rate: Option<u32>,
}
impl Options {
    pub fn parse() -> Result<Option<Self>> {
        let mut args = std::env::args().skip(1);
        let mut paths = vec![];
        let mut bpm = None;
        let mut meter = Meter::common();
        let (mut check, mut smoke, mut rate) = (false, false, None);
        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--help" | "-h" => {
                    println!(
                        "Bounded stem practice player\n\ncargo run --locked --release --example stem_player -- REFERENCE [SET_A [SET_B]] [--bpm 120] [--meter 4/4] [--check | --smoke] [--rate 48000]\n\nStarts paused. Sets require completed prototype manifests. --check prepares without audio device; --smoke exercises device playback for 8 seconds. --rate overrides preparation rate (must match device for playback).\n{}",
                        super::controls::help()
                    );
                    return Ok(None);
                }
                "--bpm" => bpm = Some(args.next().context("Missing BPM")?.parse::<f64>()?),
                "--meter" => {
                    let text = args.next().context("Missing meter")?;
                    let (n, d) = text.split_once('/').context("Use meter N/D")?;
                    meter = Meter {
                        numerator: n.parse()?,
                        denominator: d.parse()?,
                        pulse: Fraction::new(4, d.parse()?),
                        grouping: vec![],
                    };
                }
                "--rate" => rate = Some(args.next().context("Missing rate")?.parse::<u32>()?),
                "--check" => check = true,
                "--smoke" => smoke = true,
                _ if arg.starts_with('-') => anyhow::bail!("Unknown option {arg}"),
                _ => paths.push(PathBuf::from(arg)),
            }
        }
        ensure!(
            (1..=3).contains(&paths.len()),
            "Provide REFERENCE and zero, one or two stem directories; use --help"
        );
        ensure!(!(check && smoke), "Choose --check or --smoke");
        ensure!(
            bpm.is_none_or(|v| v.is_finite() && (20.0..=400.0).contains(&v)),
            "BPM must be 20–400"
        );
        ensure!(
            (1..=99).contains(&meter.numerator)
                && [1, 2, 4, 8, 16, 32, 64].contains(&meter.denominator),
            "Invalid meter"
        );
        let reference = paths.remove(0);
        Ok(Some(Self {
            reference,
            sets: paths,
            bpm,
            meter,
            check,
            smoke,
            rate,
        }))
    }
    pub fn map(&self, rate: u32, frames: u64) -> Result<Option<MeasureMap>> {
        let Some(bpm) = self.bpm else {
            return Ok(None);
        };
        let measure_frames = f64::from(rate) * 60.0 / bpm * self.meter.length().value();
        let count = (frames as f64 / measure_frames).floor() as u32;
        ensure!(
            count > 0,
            "Recording is shorter than one measure at the requested BPM/meter"
        );
        let end = (f64::from(count) * measure_frames).round() as u64;
        Ok(Some(
            MeasureMap::empty(rate, frames)
                .edit(Edit::Build {
                    start: 0,
                    end: end.min(frames),
                    count,
                    meter: self.meter.clone(),
                })?
                .map,
        ))
    }
}
