use rustfft::{Fft, FftPlanner, num_complex::Complex};
use serde::{Deserialize, Serialize};
use std::{f32::consts::PI, sync::Arc};

use crate::{cfg::AnalysisConfig, unit};

use super::{AudibleSpec, RawSpec};

#[derive(Clone)]
pub struct FftData {
    pub raw: RawSpec<Complex<f32>>,
    pub db: AudibleSpec<unit::Db>,
    pub fft: Arc<dyn Fft<f32>>,
}

impl FftData {
    pub fn blank(cfg: &AnalysisConfig) -> Self {
        Self {
            raw: RawSpec::blank_default(cfg),
            db: AudibleSpec::blank_default(cfg),
            fft: FftPlanner::new().plan_fft_forward(cfg.fft.frame_len),
        }
    }

    pub fn new(
        fft: Arc<dyn Fft<f32>>,
        cfg: &AnalysisConfig,
        samples: impl ExactSizeIterator<Item = i16>,
    ) -> Self {
        let raw = RawSpec(fft_samples(fft.as_ref(), samples));
        let db = raw
            .audible_slice(cfg)
            .into_iter()
            .map(|a| unit::Db::from_amplitude(a.norm()))
            .collect::<Vec<_>>();

        Self {
            raw,
            db: AudibleSpec(db),
            fft,
        }
    }
}

// fn hamming_window_multiplier(i: usize, len: usize) -> f32 {
//     0.54 - (0.46 * (2.0 * PI * i as f32 / f32::cos(len as f32 - 1.0)))
// }

fn hanning_window_multiplier(i: usize, len: usize) -> f32 {
    0.5 * (1.0 - f32::cos(2.0 * PI * i as f32 / (len - 1) as f32))
}

/// Runs a discrete fourier transform on a buffer of audio samples
fn fft_samples(
    fft: &dyn Fft<f32>,
    samples: impl ExactSizeIterator<Item = i16>,
) -> Vec<Complex<f32>> {
    debug_assert_eq!(fft.len(), samples.len());
    let mut buffer = samples
        .into_iter()
        .map(|i| (i as f32) / i16::MAX as f32)
        .enumerate()
        .map(|(i, sample)| sample * hanning_window_multiplier(i, fft.len()))
        .map(|re| Complex { re, im: 0.0 })
        .collect::<Vec<_>>();
    fft.process(&mut buffer);
    buffer
}

#[derive(Deserialize, Serialize)]
#[serde(default)]
pub struct FftConfig {
    pub frame_len: usize,
    pub hop_len: usize,
}

impl Default for FftConfig {
    fn default() -> Self {
        Self {
            frame_len: 4096,
            hop_len: 1024,
        }
    }
}
