use rustfft::{Fft, num_complex::Complex};
use std::{f32::consts::PI, sync::Arc};

fn hamming_window_multiplier(i: usize, len: usize) -> f32 {
    0.54 - (0.46 * (2.0 * PI * i as f32 / f32::cos(len as f32 - 1.0)))
}

/// Runs a discrete fourier transoform on a buffer of audio samples
pub fn fft_samples(fft: Arc<dyn Fft<f32>>, samples: &[i32]) -> Vec<Complex<f32>> {
    debug_assert_eq!(fft.len(), samples.len());
    let mut buffer = samples
        .into_iter()
        .map(|i| *i as f32)
        .enumerate()
        .map(|(i, sample)| sample * hamming_window_multiplier(i, fft.len()))
        .map(|re| Complex { re, im: 0.0 })
        .collect::<Vec<_>>();
    fft.process(&mut buffer);
    buffer
}
