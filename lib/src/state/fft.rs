use rustfft::{Fft, FftPlanner, num_complex::Complex};
use serde::{Deserialize, Serialize};
use std::{collections::VecDeque, f32::consts::PI, iter, sync::Arc};

use crate::{cfg::AnalysisConfig, unit, util::profile_function};

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
        profile_function!();
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
    debug_assert!(i < len);
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

pub struct InverseStft {
    fft: Arc<dyn Fft<f32>>,
    hop_len: usize,
    samples: VecDeque<Vec<f32>>,
    // buffer: VecDeque<Complex<f32>>,
}

impl InverseStft {
    pub fn new(cfg: &AnalysisConfig) -> Self {
        Self {
            fft: FftPlanner::new().plan_fft_inverse(cfg.fft.frame_len),
            hop_len: cfg.fft.hop_len,
            samples: VecDeque::from_iter(iter::repeat_n(vec![0.0; cfg.fft.frame_len], cfg.hops())),
            // buffer: VecDeque::from_iter(iter::repeat_n(Complex::ZERO, cfg.fft.frame_len)),
        }
    }

    /// Returns the samples for the frequencies given `hop_num - 1` frames ago
    #[must_use]
    pub fn push(
        &mut self,
        mut frequencies: Vec<Complex<f32>>,
    ) -> impl ExactSizeIterator<Item = i16> {
        assert_eq!(self.fft.len(), frequencies.len());

        // self.buffer.drain(0..self.hop_len);
        // self.buffer.extend(frequencies.into_iter());

        // let mut buffer = self.buffer.iter().cloned().collect::<Vec<_>>();
        self.fft.process(&mut frequencies);
        self.samples.pop_front();
        self.samples
            .push_back(frequencies.into_iter().map(|c| c.re).collect());

        let hops = self.fft.len() / self.hop_len;
        (0..self.hop_len)
            .into_iter()
            .map(move |r| {
                let mut sum = 0.0;
                let mut w_sum = 0.0;
                for n in 0..hops {
                    let i = (hops - n - 1) * self.hop_len + r;
                    let w = hann_window(i, self.fft.len());
                    sum += self.samples[n][i] * w;
                    w_sum += w * w;
                }
                sum / w_sum
            })
            .map(|f| (f * 20.0) as i16)
        // .map(|f| (f * i16::MAX as f32) as i16)
    }
}

// pub fn istft_samples(
//     fft: &dyn Fft<f32>,
//     mut frequencies: Vec<Complex<f32>>,
//     hop_len: usize,
// ) -> impl Iterator<Item = i16> {
//     assert_eq!(fft.len(), frequencies.len());

//     fft.process(&mut frequencies);

//     let hops = fft.len() / hop_len;
//     (0..hop_len)
//         .into_iter()
//         .rev()
//         .map(move |r| {
//             let mut sum = 0.0;
//             let mut w_sum = 0.0;
//             for n in 1..=hops {
//                 let i = n * hop_len - r - 1;
//                 let w = hann_window(i, fft.len());
//                 sum += frequencies[i].re * w / fft.len() as f32;
//                 w_sum += w * w;
//             }
//             sum / w_sum
//         })
//         .map(|f| (f * i16::MAX as f32) as i16)
// }

#[derive(Deserialize, Serialize)]
#[serde(default)]
pub struct FftConfig {
    pub frame_len: usize,
    pub hop_len: usize,
    pub sample_rate: usize,
}

impl Default for FftConfig {
    fn default() -> Self {
        Self {
            frame_len: 4096,
            hop_len: 1024,
            sample_rate: 44100,
        }
    }
}
