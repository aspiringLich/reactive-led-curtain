use std::iter;

use ecolor::Color32;
use tiny_skia::{Color, Pixmap};

use crate::{
    cfg::AnalysisConfig,
    color::{Oklch, OklchGradient},
    easing::EasingFunctions,
    util::profile_function,
};

use super::{light::LightData, power::PowerData};

#[derive(Clone)]
pub struct PaintData {
    pub colors: Vec<Color32>,
    pub pix: Pixmap,
    pub harmonic: Canvas<HarmonicPixel>,
}

#[derive(Clone, Copy, Default)]
pub struct HarmonicPixel {
    pub color: f32,
    pub intensity: f32,
}

struct PaintCtx<'a> {
    easing: &'a mut EasingFunctions,
    light: &'a LightData,
    power: &'a PowerData,
    w: f32,
    h: f32,
}

impl PaintData {
    pub fn blank(cfg: &AnalysisConfig) -> Self {
        Self {
            pix: Pixmap::new(cfg.light.width, cfg.light.height).unwrap(),
            colors: Vec::from_iter(iter::repeat_n(
                Color32::BLACK,
                cfg.light.width as usize * cfg.light.height as usize,
            )),
            harmonic: Canvas::new(cfg, HarmonicPixel::default()),
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

        PaintCtx {
            easing,
            light,
            power,
            w,
            h,
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

    fn percussive_background<'a>(&mut self, ctx: &mut PaintCtx<'a>) -> Canvas<Oklch> {
        let p = ctx.light.percussive.average();
        let b = ctx.light.bass_percussive.average();

        let mut canvas = Canvas::new(ctx, Oklch::TRANSPARENT);

        const FACTOR: f32 = 0.4;
        let ratio = p / (p + b + f32::EPSILON) * FACTOR * FACTOR;
        let palpha = ctx.easing.percussive.ease_normalize(p) * (1.0 + ratio);
        let ratio = p / (p + b + f32::EPSILON) * FACTOR;
        let balpha = ctx.easing.bass.ease_normalize(b) * (1.0 - ratio);

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

    fn harmonic_lines<'a>(&mut self, ctx: &mut PaintCtx<'a>, canvas: &mut Canvas<Oklch>) {
        // let mut paint = Paint::default();
        let padding = ((ctx.w - 12.0) / 2.0) as usize;
        let grad = OklchGradient::new_hex(["#ff0d17", "#ecaf3b", "#6cd74a"].into_iter());

        self.harmonic.rotate_down();
        for j in 0..12 {
            let intensity = ctx.easing.note.ease_normalize(ctx.light.notes[j].average()) * 0.9;
            let color = ctx
                .easing
                .octave
                .ease_normalize(ctx.power.average_octave[j]);
            // let color = grad.color(avg).unwrap();
            // row[j + padding] = row[j + padding].lerp(&color, o);
            self.harmonic.row(0)[j + padding] = HarmonicPixel { color, intensity };

            // ( ͡° ͜ʖ ͡°)
            let edge_factor = 0.5;
            if j < padding {
                self.harmonic.row(0)[padding + 12 + j] = HarmonicPixel {
                    color,
                    intensity: intensity * (padding - j) as f32 / padding as f32 * edge_factor,
                }
            }
            if j >= 12 - padding {
                self.harmonic.row(0)[padding + j - 12] = HarmonicPixel {
                    color,
                    intensity: intensity * (j + padding - 12) as f32 / padding as f32 * edge_factor,
                }
            }
        }

        for (rowh, rowc) in iter::repeat_n(self.harmonic.row_immut(0), 12)
            .chain(iter::repeat_n(self.harmonic.row_immut(1), 5))
            .chain(iter::repeat_n(self.harmonic.row_immut(2), 3))
            .chain(iter::repeat_n(self.harmonic.row_immut(3), 2))
            .chain(iter::repeat_n(self.harmonic.row_immut(4), 2))
            .chain(iter::repeat_n(self.harmonic.row_immut(5), 1))
            .chain(iter::repeat_n(self.harmonic.row_immut(6), 1))
            .chain(iter::repeat_n(self.harmonic.row_immut(7), 1))
            .zip(canvas.iter_rows())
        {
            for (pixelh, pixelc) in rowh.iter().zip(rowc.iter_mut()) {
                *pixelc = pixelc.lerp(&grad.color(pixelh.color).unwrap(), pixelh.intensity)
            }
        }

        // for (i, row) in canvas.iter_rows().enumerate() {
        //     for j in 0..12 {
        //         let o = ctx.easing.note.ease_normalize(ctx.light.notes[j].average()) * 0.9;
        //         let avg = ctx
        //             .easing
        //             .octave
        //             .ease_normalize(ctx.power.average_octave[j]);
        //         let color = grad.color(avg).unwrap();
        //         row[j + padding] = row[j + padding].lerp(&color, o);

        //         // ( ͡° ͜ʖ ͡°)
        //         let edge_factor = 0.5;
        //         if j < padding {
        //             row[padding + 12 + j] = row[padding + 12 + j].lerp(
        //                 &color,
        //                 o * (padding - j) as f32 / padding as f32 * edge_factor,
        //             );
        //         }
        //         if j >= 12 - padding {
        //             row[j + padding - 12] = row[j + padding - 12].lerp(
        //                 &color,
        //                 o * (j + padding - 12) as f32 / padding as f32 * edge_factor,
        //             );
        //         }
        //     }
        // }
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
pub struct Canvas<T> {
    data: Vec<T>,
    w: u32,
    h: u32,
}

trait CanvasWidthHeight {
    fn width(&self) -> u32;
    fn height(&self) -> u32;
}

impl<'a> CanvasWidthHeight for PaintCtx<'a> {
    fn width(&self) -> u32 {
        self.w as u32
    }
    fn height(&self) -> u32 {
        self.h as u32
    }
}

impl CanvasWidthHeight for AnalysisConfig {
    fn width(&self) -> u32 {
        self.light.width
    }
    fn height(&self) -> u32 {
        self.light.height
    }
}

impl<T> Canvas<T> {
    fn new<'a>(ctx: &impl CanvasWidthHeight, default: T) -> Self
    where
        T: Clone,
    {
        let w = ctx.width();
        let h = ctx.height();
        let data = Vec::from_iter(iter::repeat_n(default, w as usize * h as usize));
        Self { data, w, h }
    }

    pub fn row(&mut self, r: usize) -> &mut [T] {
        let start = r * self.w as usize;
        let end = start + self.w as usize;
        &mut self.data[start..end]
    }

    pub fn row_immut(&self, r: usize) -> &[T] {
        let start = r * self.w as usize;
        let end = start + self.w as usize;
        &self.data[start..end]
    }

    pub fn iter_rows(&mut self) -> impl ExactSizeIterator<Item = &mut [T]> {
        self.data.chunks_mut(self.w as usize)
    }

    pub fn rotate_down(&mut self) {
        self.data.rotate_right(self.w as usize);
    }
}

impl Canvas<Oklch> {
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
}
