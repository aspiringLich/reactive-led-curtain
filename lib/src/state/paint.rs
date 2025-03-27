use ecolor::Color32;
use tiny_skia::{
    Color, GradientStop, LinearGradient, Mask, Paint, Pixmap, Point, Rect, SpreadMode, Transform,
};

use crate::{cfg::AnalysisConfig, easing::EasingFunctions};

use super::{AnalysisState, light::LightData};

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

    pub fn advance(mut self, easing: &mut EasingFunctions, light: &LightData) -> Self {
        let w = self.pix.width() as f32;
        let h = self.pix.height() as f32;
        let center_top = Point::from_xy(w / 2.0, 0.0);
        let center_bottom = Point::from_xy(w / 2.0, h);
        let full = Rect::from_ltrb(0.0, 0.0, w, h).unwrap();

        // PERCUSSIVE BACKGROUND
        let p = easing.percussive.ease_normalize(light.percussive.average());
        let b = easing.percussive.ease_normalize(light.bass_percussive.average());
        let mut paint = Paint::default();
        paint.set_color(Color::WHITE);
        paint.shader = LinearGradient::new(
            center_top,
            center_bottom,
            vec![
                GradientStop::new(0.0, Color::BLACK),
                GradientStop::new(1.0, Color32::WHITE.gamma_multiply(p).into_color()),
            ],
            SpreadMode::Pad,
            Transform::identity(),
        ).unwrap();
        self.pix.fill_rect(
            full,
            &paint,
            Transform::identity(),
            None,
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
