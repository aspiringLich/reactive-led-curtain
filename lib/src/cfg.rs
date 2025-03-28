use serde::{Deserialize, Serialize};

use crate::state::{fft, hps, light};

#[derive(Deserialize, Serialize, Default)]
#[serde(default)]
pub struct AnalysisConfig {
    pub spectrogram: SpectrogramConfig,
    pub fft: fft::FftConfig,
    pub hps: hps::HpsConfig,
    pub light: light::LightConfig,
}

impl AnalysisConfig {
    pub const fn frame_duration(&self) -> f32 {
        self.fft.frame_len as f32 / self.fft.sample_rate as f32
    }

    pub const fn idx_to_hz(&self, i: usize) -> f32 {
        i as f32 / self.frame_duration()
    }

    /// Assumes a sample rate of 44.1kHz
    pub const fn hz_to_idx(&self, hz: f32) -> usize {
        (hz * self.frame_duration()) as usize
    }

    pub const fn min_idx(&self) -> usize {
        self.hz_to_idx(self.spectrogram.min_frequency)
    }

    pub const fn max_idx(&self) -> usize {
        self.hz_to_idx(self.spectrogram.max_frequency)
    }

    pub const fn idx_range(&self) -> usize {
        self.max_idx() - self.min_idx()
    }

    /// Turns an index into the raw spectrogram into an index into the audible
    /// spectrogram range
    pub fn idx_to_aidx(&self, i: usize) -> usize {
        assert!(i <= self.max_idx());
        i - self.min_idx()
    }

    pub fn max_aidx(&self) -> usize {
        self.idx_to_aidx(self.max_idx())
    }

    pub fn hops(&self) -> usize {
        self.fft.frame_len / self.fft.hop_len
    }
}

#[derive(Deserialize, Serialize)]
#[serde(default)]
pub struct SpectrogramConfig {
    pub time_width: usize,
    pub image_resolution: usize,
    pub max_frequency: f32,
    pub min_frequency: f32,
}

impl Default for SpectrogramConfig {
    fn default() -> Self {
        Self {
            time_width: 512,
            image_resolution: 512,
            min_frequency: 5.0,
            max_frequency: 8000.0,
        }
    }
}
