use garde::Validate;
use serde::Deserialize;

#[derive(Deserialize, Default, Validate)]
#[serde(default)]
#[garde(context(AnalysisConfig))]
pub struct AnalysisConfig {
    #[garde(dive)]
    pub spectrogram: SpectrogramConfig,
    #[garde(dive)]
    pub fft: FftConfig,
}

impl AnalysisConfig {
    /// Assumes a sample rate of 44.1kHz
    pub const fn idx_to_hz(&self, i: usize) -> f32 {
        i as f32 * 44_100.0 / self.fft.frame_len as f32
    }

    /// Assumes a sample rate of 44.1kHz
    pub const fn hz_to_idx(&self, hz: f32) -> usize {
        (hz / 44_100.0 * self.fft.frame_len as f32) as usize
    }

    /// Assumes a sample rate of 44.1kHz
    pub const fn hz_to_idxf(&self, hz: f32) -> f32 {
        hz / 44_100.0 * self.fft.frame_len as f32
    }

    pub fn min_idx(&self) -> usize {
        self.hz_to_idx(self.spectrogram.min_frequency)
    }

    pub fn max_idx(&self) -> usize {
        self.hz_to_idx(self.spectrogram.max_frequency)
    }
}

#[derive(Deserialize, Validate)]
#[serde(default)]
#[garde(context(AnalysisConfig))]
pub struct SpectrogramConfig {
    #[garde(range(min = 0, max = 4096))]
    pub keep_states: usize,
    #[garde(range(min = 0, max = 4096))]
    pub image_resolution: usize,
    #[garde(skip)]
    pub max_frequency: f32,
    #[garde(skip)]
    pub min_frequency: f32,
}

impl Default for SpectrogramConfig {
    fn default() -> Self {
        Self {
            keep_states: 512,
            image_resolution: 512,
            min_frequency: 5.0,
            max_frequency: 8000.0,
        }
    }
}

fn fraction_of_frame_len(len: &usize, cfg: &AnalysisConfig) -> garde::Result {
    if *len % cfg.fft.frame_len == 0 {
        Ok(())
    } else {
        Err(garde::Error::new("Expected to be divisible by `frame_len`"))
    }
}

#[derive(Deserialize, Validate)]
#[serde(default)]
#[garde(context(AnalysisConfig))]
pub struct FftConfig {
    #[garde(skip)]
    pub frame_len: usize,
    #[garde(custom(fraction_of_frame_len))]
    pub hop_len: usize,
}

impl Default for FftConfig {
    fn default() -> Self {
        Self {
            frame_len: 4096,
            hop_len: 1024,
        }
    }
}
