use crate::{
    cfg::AnalysisConfig,
    fft::{self, FftOutput},
};

#[derive(Clone)]
pub struct AnalysisState {
    pub fft_out: fft::FftOutput,
}

impl AnalysisState {
    pub fn blank(cfg: &AnalysisConfig) -> Self {
        Self {
            fft_out: FftOutput::blank(cfg),
        }
    }

    pub fn from_prev(
        cfg: &AnalysisConfig,
        prev: &AnalysisState,
        samples: impl ExactSizeIterator<Item = i16>,
    ) -> Self {
        Self {
            fft_out: fft::FftOutput::new(prev.fft_out.fft.clone(), cfg, samples),
        }
    }
}
