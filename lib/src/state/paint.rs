use std::iter;

use ecolor::Color32;
use tiny_skia::{Color, Paint, Pixmap, Point, Rect, Shader, SpreadMode, Transform};

use crate::{
    cfg::AnalysisConfig,
    color::{Oklch, OklchGradient, OklchGradientStop},
    easing::EasingFunctions,
    util::profile_function,
};

use super::{light::LightData, power::PowerData};

#[derive(Clone)]
pub struct PaintData {
    pub colors: Vec<Color32>,
    pub pix: Pixmap,
}

struct PaintCtx<'a> {
    easing: &'a mut EasingFunctions,
    light: &'a LightData,
    power: &'a PowerData,
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
            colors: Vec::from_iter(iter::repeat_n(
                Color32::BLACK,
                cfg.light.width as usize * cfg.light.height as usize,
            )),
        }
    }

    fn ctx<'a>(
        &self,
        easing: &'a mut EasingFunctions,
        light: &'a LightData,
        power: &'a PowerData,
    ) -> PaintCtx<'a> {
        let w = self.pix.width() as f32;
        let h = self.pix.height() as f32;
        let center_top = Point::from_xy(w / 2.0, 0.0);
        let center_bottom = Point::from_xy(w / 2.0, h);
        let full_rect = Rect::from_ltrb(0.0, 0.0, w, h).unwrap();

        PaintCtx {
            easing,
            light,
            power,
            w,
            h,
            center_top,
            center_bottom,
            full_rect,
        }
    }

    pub fn advance(
        mut self,
        easing: &mut EasingFunctions,
        light: &LightData,
        power: &PowerData,
    ) -> Self {
        profile_function!();
        let mut ctx = self.ctx(easing, light, power);

        self.pix.fill(Color32::BLACK.into_color());

        let mut canvas = self.percussive_background(&mut ctx);
        self.harmonic_lines(&mut ctx, &mut canvas);

        self.colors = canvas.into_rgb();
        self
    }

    fn percussive_background<'a>(&mut self, ctx: &mut PaintCtx<'a>) -> Canvas {
        let p = ctx.light.percussive.average();
        let b = ctx.light.bass_percussive.average();

        let mut canvas = Canvas::new(ctx);

        const FACTOR: f32 = 0.4;
        let ratio = p / (p + b + f32::EPSILON) * FACTOR * FACTOR;
        let palpha = ctx.easing.percussive.ease_normalize(p) * (1.0 + ratio);
        let ratio = p / (p + b + f32::EPSILON) * FACTOR;
        let balpha = ctx.easing.percussive.ease_normalize(b) * (1.0 - ratio);

        let step = (balpha - palpha) / canvas.h as f32;
        for (i, row) in canvas.iter_rows().enumerate() {
            row.fill(
                Color32::WHITE
                    .gamma_multiply(palpha + step * i as f32)
                    .into(),
            );
        }

        canvas
    }

    fn harmonic_lines<'a>(&mut self, ctx: &mut PaintCtx<'a>, canvas: &mut Canvas) {
        // let mut paint = Paint::default();
        let padding = ((ctx.w - 12.0) / 2.0) as usize;
        let grad = OklchGradient::new_hex(["#f65b53", "#8acf77"].into_iter());

        for (i, row) in canvas.iter_rows().enumerate() {
            for j in 0..12 {
                let o = ctx.easing.note.ease_normalize(ctx.light.notes[j].average());
                let avg = ctx
                    .easing
                    .octave
                    .ease_normalize(ctx.power.average_octave[j]);
                let color = grad.color(avg).unwrap();
                row[j + padding] = row[j + padding].lerp(&color, o);

                // ( ͡° ͜ʖ ͡°)
                let edge_factor = 0.5;
                if j < padding {
                    row[padding + 12 + j] = row[padding + 12 + j].lerp(
                        &color,
                        o * (padding - j) as f32 / padding as f32 * edge_factor,
                    );
                }
                if j >= 12 - padding {
                    row[j + padding - 12] = row[j + padding - 12].lerp(
                        &color,
                        o * (j + padding - 12) as f32 / padding as f32 * edge_factor,
                    );
                }
            }
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

#[derive(Clone, Debug)]
pub struct Canvas {
    data: Vec<Oklch>,
    w: u32,
    h: u32,
}

impl Canvas {
    fn new<'a>(ctx: &PaintCtx<'a>) -> Self {
        let w = ctx.w;
        let h = ctx.h;
        let data = Vec::from_iter(iter::repeat_n(Oklch::TRANSPARENT, w as usize * h as usize));
        Self {
            data,
            w: w as u32,
            h: h as u32,
        }
    }

    pub fn row(&mut self, r: usize) -> &mut [Oklch] {
        let start = r * self.w as usize;
        let end = start + self.w as usize;
        &mut self.data[start..end]
    }

    pub fn overlay(&mut self, other: &Self) {
        for i in 0..self.data.len() {
            self.data[i] = self.data[i].overlay(&other.data[i]);
        }
    }

    pub fn into_rgb(self) -> Vec<Color32> {
        self.data
            .into_iter()
            .map(|c| {
                let c: Color32 = c.into();
                Color32::from_rgb(
                    (c.r() as f32 * c.a() as f32 / 256.0) as u8,
                    (c.g() as f32 * c.a() as f32 / 256.0) as u8,
                    (c.b() as f32 * c.a() as f32 / 256.0) as u8,
                )
            })
            .collect()
    }

    pub fn iter_rows(&mut self) -> impl ExactSizeIterator<Item = &mut [Oklch]> {
        self.data.chunks_mut(self.w as usize)
    }
}
