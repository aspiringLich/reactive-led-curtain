use ecolor::Color32;
use tiny_skia::{
    Color, GradientStop, LinearGradient, Paint, Pixmap, Point, Rect, SpreadMode, Transform,
};

use crate::{cfg::AnalysisConfig, easing::EasingFunctions};

use super::light::LightData;

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

        self.pix.fill(Color32::BLACK.into_color());

        // PERCUSSIVE BACKGROUND
        let p = light.percussive.average();
        let b = light.bass_percussive.average();
        let mut paint = Paint::default();
        paint.set_color(Color32::WHITE.into_color());
        let ratio = p / (p + b + f32::EPSILON);
        const FACTOR: f32 = 0.9;
        let mut pcol = Color::WHITE;
        pcol.apply_opacity(easing.percussive.ease_normalize(p * (1.0 / FACTOR - 1.0 + ratio) * FACTOR));
        // pcol.apply_opacity(easing.percussive.ease_normalize(p));
        let mut bcol = Color::WHITE;
        bcol.apply_opacity(easing.percussive.ease_normalize(b));
        paint.shader = LinearGradient::new(
            center_top,
            center_bottom,
            vec![GradientStop::new(0.0, pcol), GradientStop::new(1.0, bcol)],
            SpreadMode::Pad,
            Transform::identity(),
        )
        .unwrap();
        self.pix
            .fill_rect(full, &paint, Transform::identity(), None);
        self
    }
}

trait Methods {
    fn into_color(&self) -> Color;
}

impl Methods for Color32 {
    fn into_color(&self) -> Color {
        Color::from_rgba8(self.r(), self.g(), self.b(), self.a())
    }
}
