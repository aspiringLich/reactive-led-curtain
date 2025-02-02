use rustfft::{Fft, FftPlanner, num_complex::Complex};
use serde::{Deserialize, Serialize};
use std::{f32::consts::PI, sync::Arc};

use crate::{cfg::AnalysisConfig, unit};

use super::{AudibleSpec, RawSpec};

#[derive(Clone)]
pub struct FftData {
    pub raw: RawSpec<Complex<f32>>,
    pub audible: AudibleSpec<Complex<f32>>,
    pub power: AudibleSpec<unit::Power>,
    pub db: AudibleSpec<unit::Db>,
    pub fft: Arc<dyn Fft<f32>>,
}

impl FftData {
    pub fn blank(cfg: &AnalysisConfig) -> Self {
        Self {
            raw: RawSpec::blank_default(cfg),
            audible: AudibleSpec::blank_default(cfg),
            power: AudibleSpec::blank_default(cfg),
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
        let audible = raw.audible_slice(cfg).into_iter().cloned().collect();
        let db = raw
            .audible_slice(cfg)
            .into_iter()
            .map(|a| a.norm().into())
            .collect();
        let magnitude = raw
            .audible_slice(cfg)
            .into_iter()
            .map(|a| unit::Power(a.norm_sqr()))
            .collect();

        Self {
            raw,
            audible: AudibleSpec(audible),
            power: AudibleSpec(magnitude),
            db: AudibleSpec(db),
            fft,
        }
    }
}

// fn hamming_window_multiplier(i: usize, len: usize) -> f32 {
//     0.54 - (0.46 * (2.0 * PI * i as f32 / f32::cos(len as f32 - 1.0)))
// }

fn hann_window(i: usize, len: usize) -> f32 {
    0.5 * (1.0 - f32::cos(2.0 * PI * i as f32 / (len - 1) as f32))
}

/// Runs a discrete fourier transform on a buffer of audio samples
pub fn fft_samples(
    fft: &dyn Fft<f32>,
    samples: impl ExactSizeIterator<Item = i16>,
) -> Vec<Complex<f32>> {
    assert_eq!(fft.len(), samples.len());
    let mut buffer = samples
        .into_iter()
        .map(|i| (i as f32) / i16::MAX as f32)
        .enumerate()
        .map(|(i, sample)| sample * hann_window(i, fft.len()))
        .map(|re| Complex { re, im: 0.0 })
        .collect::<Vec<_>>();
    fft.process(&mut buffer);
    buffer
}

pub fn istft_samples(
    fft: &dyn Fft<f32>,
    mut frequencies: Vec<Complex<f32>>,
    hop_len: usize,
) -> impl Iterator<Item = i16> {
    assert_eq!(fft.len(), frequencies.len());

    fft.process(&mut frequencies);

    let hops = fft.len() / hop_len;
    (0..hop_len)
        .into_iter()
        .map(move |r| {
            let mut sum = 0.0;
            let mut w_sum = 0.0;
            let r = hop_len - r - 1;
            for n in 1..=hops {
                let i = n * hop_len - r - 1;
                let w = hann_window(i, fft.len());
                sum += frequencies[i].re * w / fft.len() as f32;
                w_sum += w * w;
            }
            sum / w_sum
        })
        .map(|f| (f * i16::MAX as f32) as i16)
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
