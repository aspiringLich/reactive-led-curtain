use std::{collections::VecDeque, iter};

use crate::{cfg::AnalysisConfig, fft::{self, FftOutput}};

#[derive(Clone)]
pub struct AnalysisState {
    pub fft_out: fft::FftOutput,
}

impl AnalysisState {
    pub fn blank(cfg: &AnalysisConfig) -> Self {
        Self {
            fft_out: FftOutput::blank(cfg)
        }
    }

    pub fn from_prev(
        ctx: &AnalysisContext,
        prev: &AnalysisState,
        samples: impl ExactSizeIterator<Item = i16>,
    ) -> Self {
        Self {
            fft_out: fft::FftOutput::new(prev.fft_out.fft.clone(), samples),
        }
    }
}

pub struct AnalysisContext {
    pub prev_states: VecDeque<AnalysisState>,
    pub cfg: AnalysisConfig,
}

impl AnalysisContext {
    pub fn new(cfg: AnalysisConfig) -> Self {
        Self {
            prev_states: VecDeque::from_iter(
                iter::repeat(AnalysisState::blank(&cfg)).take(cfg.spectrogram.keep_states),
            ),
            cfg,
        }
    }
}
