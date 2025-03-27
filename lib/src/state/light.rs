use serde::{Deserialize, Serialize};

use crate::{cfg::AnalysisConfig, util::RollingAverage};

use super::power::{DData, PowerData};

#[derive(Clone)]
pub struct LightData {
    pub p_raw: f32,
    pub bp_raw: f32,
    pub percussive: RollingAverage,
    pub bass_percussive: RollingAverage,
}

impl LightData {
    pub fn blank(cfg: &AnalysisConfig) -> Self {
        Self {
            p_raw: 0.0,
            bp_raw: 0.0,
            percussive: RollingAverage::new(4),
            bass_percussive: RollingAverage::new(4),
        }
    }

    pub fn advance(mut self, cfg: &AnalysisConfig, power: &PowerData) -> Self {
        spiked_d_smooth(&mut self.p_raw, &power.p_filtered_power);
        spiked_d_smooth(&mut self.bp_raw, &power.p_bass_power);
        self.percussive.consume(self.p_raw - self.bp_raw);
        self.bass_percussive.consume(self.bp_raw);
        self
    }
}

fn spiked_d_smooth(l: &mut f32, d: &DData<f32>) {
    let dval = if d.dval > 0.0 { d.dval } else { 0.0 };
    *l = f32::max(0.0, *l + dval);
    *l *= 0.9;
}

#[derive(Deserialize, Serialize)]
#[serde(default)]
pub struct LightConfig {
    pub width: u32,
    pub height: u32,
}

impl Default for LightConfig {
    fn default() -> Self {
        Self {
            width: 20,
            height: 26,
        }
    }
}

// #[derive(Clone, Copy)]
// pub struct LogVal {
//     val: f32,
// }

// impl Default for LogVal {
//     fn default() -> Self {
//         Self::new(0.0)
//     }
// }

// impl LogVal {
//     pub fn new(val: f32) -> Self {
//         Self { val }
//     }

//     pub fn val(&self) -> f32 {
//         self.val
//     }

//     pub fn val_mut(&mut self) -> &mut f32 {
//         &mut self.val
//     }

//     pub fn log(&self) -> f32 {
//         f32::log2(self.val + 1.0)
//     }
// }
