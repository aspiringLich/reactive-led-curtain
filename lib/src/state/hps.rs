use serde::{Deserialize, Serialize};

use crate::{cfg::AnalysisConfig, unit::{Db, Power}};

use super::{AudibleSpec, fft::FftData};

pub struct HpsData {
    pub past_magnitudes: AudibleSpec<median::Filter<Power>>,
    pub h_enhanced: AudibleSpec<Power>,
    pub p_enhanced: AudibleSpec<Power>,
    pub harmonic: AudibleSpec<Db>,
    pub percussive: AudibleSpec<Db>,
    pub residual: AudibleSpec<Db>,
}

impl HpsData {
    pub fn blank(cfg: &AnalysisConfig) -> Self {
        Self {
            past_magnitudes: AudibleSpec::blank_clone(&median::Filter::new(cfg.hps.h_filter_span), cfg),
            h_enhanced: AudibleSpec::blank_default(cfg),
            p_enhanced: AudibleSpec::blank_default(cfg),
            harmonic: AudibleSpec::blank_default(cfg),
            percussive: AudibleSpec::blank_default(cfg),
            residual: AudibleSpec::blank_default(cfg),
        }
    }

    pub fn advance(mut self, cfg: &AnalysisConfig, fft: &FftData) -> Self {
        let hps = &cfg.hps;

        for (i, filter) in self.past_magnitudes.iter_mut().enumerate() {
            filter.consume(fft.power[i]);
        }

        self.h_enhanced = AudibleSpec(self.past_magnitudes.iter().map(|buf| buf.median()).collect());
        let mut filter = median::Filter::new(hps.p_filter_span);
        self.p_enhanced = AudibleSpec(
            fft.power
                .iter()
                .map(|d| {
                    filter.consume(*d);
                    filter.median()
                })
                .collect(),
        );

        for (i, db) in fft.db.iter().enumerate() {
            let separation = *self.h_enhanced[i] / (*self.p_enhanced[i] + f32::EPSILON);
            if separation > hps.separation_factor {
                self.harmonic[i] = *db;
                self.percussive[i] = Db::default();
                self.residual[i] = Db::default();
            } else if separation < 1.0 / hps.separation_factor {
                self.harmonic[i] = Db::default();
                self.percussive[i] = *db;
                self.residual[i] = Db::default();
            } else {
                self.harmonic[i] = Db::default();
                self.percussive[i] = Db::default();
                self.residual[i] = *db;
            }
        }

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
