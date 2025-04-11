use ecolor::Color32;
use tiny_skia::{
    Color, GradientStop, LinearGradient, Paint, Pixmap, Point, Rect, Shader, SpreadMode, Transform
};

use crate::{cfg::AnalysisConfig, easing::EasingFunctions, util::profile_function};

use super::light::LightData;

#[derive(Clone)]
pub struct PaintData {
    pub pix: Pixmap,
}

struct PaintCtx<'a> {
    easing: &'a mut EasingFunctions,
    light: &'a LightData,
    w: f32,
    h: f32,
    center_top: Point,
    center_bottom: Point,
    full_rect: Rect,
}

impl<'a> PaintCtx<'a> {
    fn vrect(&self, x: f32) -> Rect {
        let w = self.w;
        let h = self.h;
        Rect::from_ltrb(x, 0.0, x + 1.0, h).unwrap()
    }
}

impl PaintData {
    pub fn blank(cfg: &AnalysisConfig) -> Self {
        Self {
            pix: Pixmap::new(cfg.light.width, cfg.light.height).unwrap(),
        }
    }

    fn ctx<'a>(&self, easing: &'a mut EasingFunctions, light: &'a LightData) -> PaintCtx<'a> {
        let w = self.pix.width() as f32;
        let h = self.pix.height() as f32;
        let center_top = Point::from_xy(w / 2.0, 0.0);
        let center_bottom = Point::from_xy(w / 2.0, h);
        let full_rect = Rect::from_ltrb(0.0, 0.0, w, h).unwrap();

        PaintCtx {
            easing,
            light,
            w,
            h,
            center_top,
            center_bottom,
            full_rect,
        }
    }

    pub fn advance(mut self, easing: &mut EasingFunctions, light: &LightData) -> Self {
        profile_function!();
        let mut ctx = self.ctx(easing, light);

        self.pix.fill(Color32::BLACK.into_color());

        self.percussive_background(&mut ctx);
        self.harmonic_lines(&mut ctx);

        self
    }

    fn percussive_background<'a>(&mut self, ctx: &mut PaintCtx<'a>) {
        let p = ctx.light.percussive.average();
        let b = ctx.light.bass_percussive.average();
        let mut paint = Paint::default();
        paint.set_color(Color32::WHITE.into_color());
        const FACTOR: f32 = 0.3;
        let ratio = p / (p + b + f32::EPSILON) * FACTOR * FACTOR;
        let mut pcol = Color::WHITE;
        pcol.apply_opacity(ctx.easing.percussive.ease_normalize(p) * (1.0 + ratio));
        // pcol.apply_opacity(easing.percussive.ease_normalize(p));
        let mut bcol = Color::WHITE;
        let ratio = p / (p + b + f32::EPSILON) * FACTOR;
        bcol.apply_opacity(ctx.easing.percussive.ease_normalize(b) * (1.0 - ratio));
        // bcol.apply_opacity(easing.percussive.ease_normalize(b));
        paint.shader = LinearGradient::new(
            ctx.center_top,
            ctx.center_bottom,
            vec![GradientStop::new(0.0, pcol), GradientStop::new(1.0, bcol)],
            SpreadMode::Pad,
            Transform::identity(),
        )
        .unwrap();
        self.pix
            .fill_rect(ctx.full_rect, &paint, Transform::identity(), None);
    }

    fn harmonic_lines<'a>(&mut self, ctx: &mut PaintCtx<'a>) {
        let mut paint = Paint::default();
        let padding = ((ctx.w - 12.0) / 2.0).round();

        for i in 0..12 {
            let o = ctx.easing.octave.ease_normalize(ctx.light.octave[i].average());
            paint.shader = Shader::SolidColor(Color32::RED.gamma_multiply(o).into_color());
            self.pix.fill_rect(ctx.vrect(padding + i as f32), &paint, Transform::identity(), None);
        }
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
