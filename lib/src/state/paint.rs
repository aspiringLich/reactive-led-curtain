use ecolor::Color32;
use tiny_skia::{Color, Paint, Pixmap};

use crate::cfg::AnalysisConfig;

use super::{light::LightData, AnalysisState};

#[derive(Clone)]
pub struct PaintData {
    pub pix: Pixmap,
}

impl PaintData {
    pub fn blank(cfg: &AnalysisConfig) -> Self {
        Self {
            pix: Pixmap::new(cfg.light.width, cfg.light.height).unwrap(),
        }
    }

    pub fn advance(mut self, light: &LightData) -> Self {
        self.pix.fill(
            Color32::WHITE.gamma_multiply(light.percussive.average().max(0.0) / 100.0).into_color(),
        );
        self
    }
}

trait IntoColor {
    fn into_color(&self) -> Color;
}

impl IntoColor for Color32 {
    fn into_color(&self) -> Color {
        Color::from_rgba8(self.r(), self.g(), self.b(), self.a())
    }
}
