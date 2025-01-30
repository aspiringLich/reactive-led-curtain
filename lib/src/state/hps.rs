use serde::{Deserialize, Serialize};

use crate::{cfg::AnalysisConfig, unit::Db};

use super::{AudibleSpec, fft::FftData};

pub struct HpsData {
    pub past_db: AudibleSpec<median::Filter<Db>>,
    pub h_enhanced: AudibleSpec<Db>,
    pub p_enhanced: AudibleSpec<Db>,
}

impl HpsData {
    pub fn blank(cfg: &AnalysisConfig) -> Self {
        Self {
            past_db: AudibleSpec::blank_clone(&median::Filter::new(cfg.hps.h_filter_span), cfg),
            h_enhanced: AudibleSpec::blank_default(cfg),
            p_enhanced: AudibleSpec::blank_default(cfg),
        }
    }

    pub fn advance(mut self, cfg: &AnalysisConfig, fft: &FftData) -> Self {
        let hps = &cfg.hps;

        for (i, filter) in self.past_db.iter_mut().enumerate() {
            filter.consume(fft.db[i]);
        }

        self.h_enhanced = AudibleSpec(self.past_db.iter().map(|buf| buf.median()).collect());
        let mut filter = median::Filter::new(hps.p_filter_span);
        self.p_enhanced = AudibleSpec(
            fft.db
                .iter()
                .map(|d| {
                    filter.consume(*d);
                    filter.median()
                })
                .collect(),
        );

        self
    }
}

#[derive(Deserialize, Serialize)]
#[serde(default)]
pub struct HpsConfig {
    pub p_filter_span: usize,
    pub h_filter_span: usize,
    pub separation_factor: f32,
}

impl Default for HpsConfig {
    fn default() -> Self {
        Self {
            // 44.1k samples/s * 0.2s / 1024 samples/hop
            //  = 8.61328125
            p_filter_span: 9,
            // (max_idx / max_freq) idx/hz * 500`hz
            //  = ((8000 / 44_100 * 4096) / 8000) * 500
            //  = 46.43990929705215 indices
            h_filter_span: 46,
            separation_factor: 2.0,
        }
    }
}
