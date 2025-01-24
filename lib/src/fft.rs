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
    pub fn new(fft: Arc<dyn Fft<f32>>, samples: &[i16]) -> Self {
        let raw = fft_samples(fft.as_ref(), samples);
        let db = raw
            .iter()
            .into_iter()
            .map(|a| unit::Db::from_amplitude(a.norm()))
            .collect::<Vec<_>>();

        Self {
            raw,
            db,
            fft,
        }
    }
}

/// Assumes a sample rate of 44.1kHz
pub const fn idx_to_hz(i: usize) -> f32 {
    i as f32 * 44_100.0 / SAMPLE_SIZE as f32
}

/// Assumes a sample rate of 44.1kHz
pub const fn hz_to_idx(hz: f32) -> usize {
    (hz / 44_100.0 * SAMPLE_SIZE as f32) as usize
}

fn hamming_window_multiplier(i: usize, len: usize) -> f32 {
    0.54 - (0.46 * (2.0 * PI * i as f32 / f32::cos(len as f32 - 1.0)))
}

/// Runs a discrete fourier transform on a buffer of audio samples
fn fft_samples(fft: &dyn Fft<f32>, samples: &[i16]) -> Vec<Complex<f32>> {
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
