use derive_more::derive::{Deref, DerefMut};

use crate::{cfg::AnalysisConfig, util::vec_default};

pub mod fft;
pub mod hps;

#[derive(Clone)]
pub struct AnalysisState {
    pub fft_out: fft::FftData,
}

impl AnalysisState {
    pub fn blank(cfg: &AnalysisConfig) -> Self {
        Self {
            fft_out: fft::FftData::blank(cfg),
        }
    }

    pub fn from_prev(
        cfg: &AnalysisConfig,
        prev: &AnalysisState,
        samples: impl ExactSizeIterator<Item = i16>,
    ) -> Self {
        Self {
            fft_out: fft::FftData::new(prev.fft_out.fft.clone(), cfg, samples),
        }
    }
}

/// Raw spectrogram data right out of &Fft
#[derive(Deref, DerefMut, Clone, Debug, Default)]
pub struct RawSpec<T>(Vec<T>);

impl<T: Default> RawSpec<T> {
    pub fn blank_default(cfg: &AnalysisConfig) -> Self {
        Self(vec_default(cfg.fft.frame_len))
    }
}

impl<T> RawSpec<T> {
    pub fn audible_slice(&self, cfg: &AnalysisConfig) -> &[T] {
        &self.0[cfg.min_idx()..cfg.max_idx()]
    }
}

/// The audible slice of the spectrogram's frequencies
#[derive(Deref, DerefMut, Clone, Debug, Default)]
pub struct AudibleSpec<T>(Vec<T>);

impl<T: Default> AudibleSpec<T> {
    pub fn blank_default(cfg: &AnalysisConfig) -> Self {
        Self(vec_default(cfg.max_aidx()))
    }
}
