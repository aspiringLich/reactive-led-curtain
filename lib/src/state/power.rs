use std::ops::Sub;

use crate::{cfg::AnalysisConfig, unit::Power, util::RollingAverage};

use super::{AudibleSpec, hps::HpsData};

#[derive(Clone)]
pub struct PowerData {
    pub h_power_raw: f32,
    pub r_power_raw: f32,
    pub dr: f32,

    pub p_power_raw: DData<f32>,
    // pub dp: f32,
    pub p_filtered: AudibleSpec<Power>,
    pub p_filtered_power: DData<f32>,
    pub p_bass_power: DData<f32>,
    // pub dp_filtered: f32,
    pub ratio_h_p: RollingAverage,
}

impl PowerData {
    pub fn blank(cfg: &AnalysisConfig) -> Self {
        Self {
            h_power_raw: 0.0,
            r_power_raw: 0.0,
            dr: 0.0,
            p_power_raw: Default::default(),
            p_filtered: AudibleSpec::blank_default(cfg),
            p_filtered_power: Default::default(),
            p_bass_power: Default::default(),
            ratio_h_p: RollingAverage::new(5),
        }
    }

    pub fn new(cfg: &AnalysisConfig, data: &HpsData, prev: PowerData) -> Self {
        let h_power_raw = data.harmonic.power(cfg);
        let r_power_raw = data.residual.power(cfg);
        let dr = r_power_raw - prev.r_power_raw;

        let p_power_raw = data.percussive.power(cfg);

        let mut filter = median::Filter::<Power>::new(40);
        let p_filtered = AudibleSpec(
            data.percussive
                .iter()
                .map(|&d| {
                    filter.consume(d.into());
                    filter.median()
                })
                .collect(),
        );
        let p_bass_power = AudibleSpec(
            p_filtered.0[0..10]
                .iter()
                .enumerate()
                .map(|(i, &x)| Power(*x * f32::min((10 - i) as f32 / 6.0, 1.0)))
                .collect(),
        )
        .power(cfg);
        let p_filtered_power = p_filtered.power(cfg) - p_bass_power;

        let mut ratio_h_p = prev.ratio_h_p;
        ratio_h_p.consume(ratio(h_power_raw, p_filtered_power));

        Self {
            h_power_raw,
            r_power_raw,
            dr,
            p_power_raw: prev.p_power_raw.advance(p_power_raw),
            p_filtered_power: prev.p_filtered_power.advance(p_filtered_power),
            p_bass_power: prev.p_bass_power.advance(p_bass_power),
            p_filtered,
            ratio_h_p,
        }
    }
}

fn ratio(a: f32, b: f32) -> f32 {
    a / (a.abs() + b.abs() + 1e-4) * b.signum()
}

trait DataAdvance<T: Clone> {
    fn advance(&self, new: T) -> Self;
}

#[derive(Clone)]
pub struct DData<T: Sub<T, Output = T> + Clone> {
    pub val: T,
    pub dval: T,
    pub ddval: T,
}

impl<T: Sub<T, Output = T> + Default + Clone> Default for DData<T> {
    fn default() -> Self {
        Self {
            val: Default::default(),
            dval: Default::default(),
            ddval: Default::default(),
        }
    }
}

impl<T: Sub<T, Output = T> + Clone> DataAdvance<T> for DData<T> {
    fn advance(&self, new: T) -> Self {
        let dval = new.to_owned() - self.val.to_owned();
        let ddval = self.dval.to_owned() - dval.clone();
        Self {
            val: new.to_owned(),
            dval,
            ddval,
        }
    }
}

impl<T: Sub<T, Output = T> + Clone + Into<f32>> Into<f32> for DData<T> {
    fn into(self) -> f32 {
        self.val.into()
    }
}

impl<T: Sub<T, Output = T> + Clone + Into<f64>> Into<f64> for DData<T> {
    fn into(self) -> f64 {
        self.val.into()
    }
}
