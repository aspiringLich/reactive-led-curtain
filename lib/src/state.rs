use crate::fft;

#[derive(Default)]
pub struct AnalysisState {
    pub fft_out: fft::FftOutput,
}

impl AnalysisState {
    pub fn from_prev(prev: &AnalysisState, samples: impl ExactSizeIterator<Item = i16>) -> Self {
        Self {
            fft_out: fft::FftOutput::new(prev.fft_out.fft.clone(), samples),
        }
    }
}
