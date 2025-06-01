use std::iter;

use ecolor::Color32;
use serde::{Deserialize, Serialize};
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
    pub power: f32,
    pub factor: f32,
}

impl HarmonicPixel {
    pub fn lerp(&self, other: &Self, t: f32) -> Self {
        let tp = 1.0 - t;
        Self {
            color: self.color * tp + other.color * t,
            power: self.power * tp + other.power * t,
            factor: self.factor * tp + other.factor * t,
        }
    }
}

struct PaintCtx<'a> {
    cfg: &'a PaintConfig,
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
            harmonic: Canvas::new_with_size(
                cfg.light.width,
                cfg.paint.roll_len.len() as u32,
                HarmonicPixel::default(),
            ),
        }
    }

    fn ctx<'a>(
        &self,
        cfg: &'a PaintConfig,
        easing: &'a mut EasingFunctions,
        light: &'a LightData,
        power: &'a PowerData,
    ) -> PaintCtx<'a> {
        let w = self.pix.width() as f32;
        let h = self.pix.height() as f32;

        PaintCtx {
            cfg,
            easing,
            light,
            power,
            w,
            h,
        }
    }

    pub fn advance(
        mut self,
        cfg: &PaintConfig,
        easing: &mut EasingFunctions,
        light: &LightData,
        power: &PowerData,
    ) -> Self {
        profile_function!();
        let mut ctx = self.ctx(cfg, easing, light, power);

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
                    .gamma_multiply((palpha + step * i as f32) * 0.8)
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
            let power = ctx.light.notes[j].average();
            let color = ctx
                .easing
                .octave
                .ease_normalize(ctx.power.average_octave[j]);
            // let color = grad.color(avg).unwrap();
            // row[j + padding] = row[j + padding].lerp(&color, o);
            self.harmonic.row(0)[j + padding] = HarmonicPixel {
                color,
                power,
                factor: 1.0,
            };

            // ( ͡° ͜ʖ ͡°)
            let edge_factor = 0.5;
            if j < padding {
                self.harmonic.row(0)[padding + 12 + j] = HarmonicPixel {
                    color,
                    power,
                    factor: (padding - j) as f32 / padding as f32 * edge_factor,
                }
            }
            if j >= 12 - padding {
                self.harmonic.row(0)[padding + j - 12] = HarmonicPixel {
                    color,
                    power,
                    factor: (j + padding - 12) as f32 / padding as f32 * edge_factor,
                }
            }
        }

        let mut i = 0;
        let mut previous: Option<(&[HarmonicPixel], f32)> = None;
        for ((&len, &opacity), rowh) in ctx
            .cfg
            .roll_len
            .iter()
            .zip(ctx.cfg.roll_opacity.iter().chain(iter::repeat(&1.0)))
            .zip(self.harmonic.iter_rows())
        {
            for j in 0..len {
                for k in 0..rowh.len() {
                    let pixelh = &rowh[k];
                    let pixelc = &mut canvas.row(i)[k];

                    let (pixelh, opacity) = if let Some((p_rowh, p_opacity)) = previous && len != 1 {
                        let t = j as f32 / len as f32;
                        (
                            &pixelh.lerp(&p_rowh[k], 1.0 - t),
                            opacity * t + p_opacity * (1.0 -t),
                        )
                    } else {
                        (pixelh, opacity)
                    };

                    let x = ctx.easing.note.ease_normalize(pixelh.power * opacity) * pixelh.factor * 0.9;
                    *pixelc = pixelc.lerp(
                        &grad.color(pixelh.color).unwrap(),
                        x,
                    );
                }
                i += 1;
            }
            previous = Some((&*rowh, opacity));
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

    fn new_with_size(w: u32, h: u32, default: T) -> Self
    where
        T: Clone,
    {
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

#[derive(Deserialize, Serialize, Clone)]
#[serde(default)]
pub struct PaintConfig {
    pub roll_len: Vec<u32>,
    pub roll_opacity: Vec<f32>,
}

impl Default for PaintConfig {
    fn default() -> Self {
        Self {
            roll_len: vec![12, 5, 3, 2, 2, 1, 1],
            roll_opacity: vec![1.0, 0.9, 0.8, 0.7, 0.6, 0.5, 0.4],
        }
    }
}
