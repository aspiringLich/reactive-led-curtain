use crate::cfg::AnalysisConfig;

use super::hps::HpsData;


pub struct PowerData {
    pub h_power_raw: f32,
    pub p_power_raw: f32,
    pub r_power_raw: f32,
    pub dp: f32,
}

impl PowerData {
    pub fn blank(cfg: &AnalysisConfig) -> Self {
        Self {
            h_power_raw: 0.0,
            p_power_raw: 0.0,
            r_power_raw: 0.0,
            dp: 0.0,
        }
    }

    pub fn new(cfg: &AnalysisConfig, data: &HpsData, prev: &PowerData) -> Self {
        let h_power_raw = data.harmonic.power(cfg);
        let p_power_raw = data.percussive.power(cfg);
        let r_power_raw = data.residual.power(cfg);
        let dp = p_power_raw - prev.p_power_raw;

        Self {
            h_power_raw,
            p_power_raw,
            r_power_raw,
            dp,
        }
    }
}
