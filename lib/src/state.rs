use crate::fft;

#[derive(Default)]
pub struct AnalysisState {
    pub fft_out: fft::FftOutput,
}

impl AnalysisState {    
    pub fn from_prev(prev: &AnalysisState, samples: &[i16]) -> Self {
        Self {
            fft_out: fft::FftOutput::from_prev(&prev.fft_out, samples),
        }
    }
}
