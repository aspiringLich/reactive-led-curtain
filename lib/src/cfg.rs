use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Default)]
#[serde(default)]
pub struct AnalysisConfig {
    pub spectrogram: SpectrogramConfig,
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

#[derive(Deserialize, Serialize)]
#[serde(default)]
pub struct SpectrogramConfig {
    pub keep_states: usize,
    pub image_resolution: usize,
    pub max_frequency: f32,
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

#[derive(Deserialize, Serialize )]
#[serde(default)]
pub struct FftConfig {
    pub frame_len: usize,
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
