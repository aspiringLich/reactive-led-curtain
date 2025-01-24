use rustfft::{Fft, FftPlanner, num_complex::Complex};
use std::{f32::consts::PI, sync::Arc};

use crate::{SAMPLE_SIZE, unit};

pub struct FftOutput {
    pub raw: Vec<Complex<f32>>,
    pub db: Vec<unit::Db>,
    pub fft: Arc<dyn Fft<f32>>,
}

impl Default for FftOutput {
    fn default() -> Self {
        Self {
            raw: vec![Default::default(); SAMPLE_SIZE],
            db: vec![Default::default(); SAMPLE_SIZE],
            fft: FftPlanner::new().plan_fft_forward(SAMPLE_SIZE),
        }
    }
}

impl FftOutput {
    pub fn from_prev(prev: &FftOutput, samples: &[i16]) -> Self {
        let raw = fft_samples(prev.fft.clone(), samples);
        let db = raw
            .iter()
            .into_iter()
            .map(|a| unit::Db::from_amplitude(a.norm()))
            .collect::<Vec<_>>();

        Self { raw, db, fft: prev.fft.clone() }
    }
}

fn hamming_window_multiplier(i: usize, len: usize) -> f32 {
    0.54 - (0.46 * (2.0 * PI * i as f32 / f32::cos(len as f32 - 1.0)))
}

/// Runs a discrete fourier transform on a buffer of audio samples
pub fn fft_samples(fft: Arc<dyn Fft<f32>>, samples: &[i16]) -> Vec<Complex<f32>> {
    debug_assert_eq!(fft.len(), samples.len());
    let mut buffer = samples
        .into_iter()
        .map(|i| (*i as f32) / i16::MAX as f32)
        .enumerate()
        .map(|(i, sample)| sample * hamming_window_multiplier(i, fft.len()))
        .map(|re| Complex { re, im: 0.0 })
        .collect::<Vec<_>>();
    fft.process(&mut buffer);
    buffer
}
