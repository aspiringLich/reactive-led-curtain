use ebur128::EbuR128;
use serde::{Deserialize, Serialize};

use crate::util::profile_function;

#[derive(Default, Clone)]
pub struct LoudnessData {
    pub st: f64,
    pub m: f64,
    pub gain: f64,
}

#[derive(Deserialize, Serialize)]
#[serde(default)]
pub struct LoudnessConfig {
    pub target_lufs: f64,
    pub factor: f64,
}

impl LoudnessConfig {
    pub fn gain(&self, loudness: f64) -> f64 {
        10f64.powf((self.target_lufs - loudness) / 20.0 * self.factor)
    }

    /// https://github.com/sdroege/ebur128/blob/main/examples/normalize.rs
    pub fn normalize(
        &self,
        samples: impl ExactSizeIterator<Item = i16>,
        ebur: &mut EbuR128,
    ) -> impl ExactSizeIterator<Item = i16> {
        profile_function!();

        let samples = samples.collect::<Vec<_>>();
        ebur.add_frames_i16(&samples).unwrap();
        let loudness = ebur.loudness_shortterm().unwrap();

        samples
            .into_iter()
            .map(move |s| (s as f64 * self.gain(loudness)).clamp(0.0, i16::MAX as f64) as i16)
    }

    pub fn data(&self, ebur: &EbuR128) -> LoudnessData {
        let st = ebur.loudness_shortterm().unwrap();
        LoudnessData {
            st,
            m: ebur.loudness_momentary().unwrap(),
            gain: self.gain(st)
        }
    }
}

impl Default for LoudnessConfig {
    fn default() -> Self {
        Self {
            target_lufs: -20.0,
            factor: 0.4,
        }
    }
}
