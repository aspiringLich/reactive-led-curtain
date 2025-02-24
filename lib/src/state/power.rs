use crate::{cfg::AnalysisConfig, unit::Power};

use super::{AudibleSpec, hps::HpsData};

pub struct PowerData {
    pub h_power_raw: f32,
    pub p_power_raw: f32,
    pub r_power_raw: f32,
    pub dp: f32,
    pub p_filtered: AudibleSpec<Power>,
    pub p_filtered_power: f32,
    pub dp_filtered: f32,
}

impl PowerData {
    pub fn blank(cfg: &AnalysisConfig) -> Self {
        Self {
            h_power_raw: 0.0,
            p_power_raw: 0.0,
            r_power_raw: 0.0,
            dp: 0.0,
            p_filtered: AudibleSpec::blank_default(cfg),
            p_filtered_power: 0.0,
            dp_filtered: 0.0,
        }
    }

    pub fn new(cfg: &AnalysisConfig, data: &HpsData, prev: &PowerData) -> Self {
        let h_power_raw = data.harmonic.power(cfg);
        let p_power_raw = data.percussive.power(cfg);
        let r_power_raw = data.residual.power(cfg);
        let dp = p_power_raw - prev.p_power_raw;

        let mut filter = median::Filter::<Power>::new(5);
        let p_filtered = AudibleSpec(
            data.percussive
                .iter()
                .map(|&d| {
                    filter.consume(d.into());
                    filter.median()
                })
                .collect(),
        );
        let p_filtered_power = p_filtered.power(cfg);
        let dp_filtered = p_filtered_power - prev.p_filtered_power;

        Self {
            h_power_raw,
            p_power_raw,
            r_power_raw,
            dp,
            p_filtered_power,
            p_filtered,
            dp_filtered,
        }
    }
}
