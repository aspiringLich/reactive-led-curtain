use derive_more::derive::{Deref, DerefMut};
use rustfft::num_complex::Complex;

use crate::{
    cfg::AnalysisConfig,
    unit::{Db, Power},
    util::{vec_clone, vec_default},
};

pub mod fft;
pub mod hps;
pub mod light;
pub mod power;

pub struct AnalysisState {
    pub fft: fft::FftData,
    pub hps: hps::HpsData,
    pub power: power::PowerData,
}

impl AnalysisState {
    pub fn blank(cfg: &AnalysisConfig) -> Self {
        Self {
            fft: fft::FftData::blank(cfg),
            hps: hps::HpsData::blank(cfg),
            power: power::PowerData::blank(cfg),
        }
    }

    pub fn from_prev(
        cfg: &AnalysisConfig,
        prev: AnalysisState,
        samples: impl ExactSizeIterator<Item = i16>,
    ) -> Self {
        let fft = fft::FftData::new(prev.fft.fft.clone(), cfg, samples);
        let hps = prev.hps.advance(cfg, &fft);
        let power = power::PowerData::new(cfg, &hps, &prev.power);
        Self { hps, fft, power }
    }
}

/// Raw spectrogram data right out of &Fft
#[derive(Deref, DerefMut, Clone, Debug, Default)]
pub struct RawSpec<T>(pub Vec<T>);

impl<T: Default> RawSpec<T> {
    pub fn blank_default(cfg: &AnalysisConfig) -> Self {
        Self(vec_default(cfg.fft.frame_len))
    }
}
impl<T: Clone> RawSpec<T> {
    pub fn blank_clone(elem: &T, cfg: &AnalysisConfig) -> Self {
        Self(vec_clone(elem, cfg.fft.frame_len))
    }
}

impl<T> RawSpec<T> {
    pub fn audible_slice(&self, cfg: &AnalysisConfig) -> &[T] {
        &self.0[cfg.min_idx()..cfg.max_idx()]
    }

    pub fn audible_slice_mut(&mut self, cfg: &AnalysisConfig) -> &mut [T] {
        &mut self.0[cfg.min_idx()..cfg.max_idx()]
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
impl<T: Clone> AudibleSpec<T> {
    pub fn blank_clone(elem: &T, cfg: &AnalysisConfig) -> Self {
        Self(vec_clone(elem, cfg.max_aidx()))
    }
}

impl<T> AudibleSpec<T> {
    pub fn from_iter(iter: impl Iterator<Item = T>, cfg: &AnalysisConfig) -> Self {
        let v = Vec::from_iter(iter);
        debug_assert_eq!(v.len(), cfg.max_aidx());
        Self(v)
    }
}

impl AudibleSpec<Complex<f32>> {
    pub fn power(&self, cfg: &AnalysisConfig) -> f32 {
        self.iter().map(|&a| a.norm_sqr()).sum::<f32>() / cfg.fft.frame_len as f32
    }
}

impl AudibleSpec<Power> {
    pub fn power(&self, cfg: &AnalysisConfig) -> f32 {
        self.iter().map(|&a| *a).sum::<f32>() / cfg.fft.frame_len as f32
    }
}

impl<T: Into<Db> + Copy> AudibleSpec<T> {
    pub fn into_db(&self) -> AudibleSpec<Db> {
        AudibleSpec(self.iter().map(|a| (*a).into()).collect())
    }
}
