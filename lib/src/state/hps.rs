use rustfft::num_complex::Complex;
use serde::{Deserialize, Serialize};

use crate::{cfg::AnalysisConfig, unit::Power};

use super::{AudibleSpec, fft::FftData};

pub struct HpsData {
    pub past_magnitudes: AudibleSpec<median::Filter<Power>>,
    pub h_enhanced: AudibleSpec<Power>,
    pub p_enhanced: AudibleSpec<Power>,
    pub harmonic: AudibleSpec<Complex<f32>>,
    pub percussive: AudibleSpec<Complex<f32>>,
    pub residual: AudibleSpec<Complex<f32>>,
}

impl HpsData {
    pub fn blank(cfg: &AnalysisConfig) -> Self {
        let a_len = cfg.max_aidx();
        Self {
            past_magnitudes: AudibleSpec::from_iter(
                (0..a_len)
                    .into_iter()
                    .map(|i| i as f32 / a_len as f32)
                    .map(|f| f + 0.5)
                    .map(|f| median::Filter::new((f * cfg.hps.h_filter_span as f32) as usize)),
                cfg,
            ),
            h_enhanced: AudibleSpec::blank_default(cfg),
            p_enhanced: AudibleSpec::blank_default(cfg),
            harmonic: AudibleSpec::blank_default(cfg),
            percussive: AudibleSpec::blank_default(cfg),
            residual: AudibleSpec::blank_default(cfg),
        }
    }

    pub fn advance(mut self, cfg: &AnalysisConfig, fft: &FftData) -> Self {
        let hps = &cfg.hps;

        self.past_magnitudes.mutate(|i, filter| {
            filter.consume(fft.power[i]);
        });

        self.h_enhanced = AudibleSpec(
            self.past_magnitudes
                .iter()
                .map(|buf| buf.median())
                .collect(),
        );
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

        struct Mask {
            mask_h: f32,
            mask_p: f32,
        }

        let masks = fft
            .audible
            .iter()
            .enumerate()
            .map(|(i, _)| {
                let h = *self.h_enhanced[i];
                let p = *self.p_enhanced[i];
                let e = f32::EPSILON;
                let mask_h = ((h + e) / (h + p + e + e)).powf(hps.h_factor);
                let mask_p = ((p + e) / (h + p + e + e)).powf(hps.p_factor);
                Mask { mask_h, mask_p }
            })
            .collect::<Vec<_>>();

        self.harmonic
            .update(|i, _| fft.audible[i] * masks[i].mask_h);
        self.percussive
            .update(|i, _| fft.audible[i] * masks[i].mask_p);
        self.residual
            .update(|i, _| fft.audible[i] * (1.0 - masks[i].mask_h - masks[i].mask_p));

        self
    }
}

#[derive(Deserialize, Serialize)]
#[serde(default)]
pub struct HpsConfig {
    pub p_filter_span: usize,
    pub h_filter_span: usize,
    pub p_factor: f32,
    pub h_factor: f32,
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
            p_factor: 2.0,
            h_factor: 2.0,
        }
    }
}
