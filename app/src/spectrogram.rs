use std::{collections::VecDeque, iter::repeat, sync::mpsc::Receiver};

use egui::{
    Color32, ColorImage, ComboBox, Context, Image, Slider, TextureHandle, TextureOptions, Ui,
};

use lib::{SAMPLE_SIZE, SAMPLE_WINDOWS, fft, state::AnalysisState};
use serde::{Deserialize, Serialize};

use crate::cmap;

#[derive(Serialize, Deserialize)]
pub struct SpecConfig {
    db_min: f32,
    db_max: f32,
    scale: SpecScale,
}

#[derive(Serialize, Deserialize, PartialEq)]
pub enum SpecScale {
    Linear,
    Log,
}

impl Default for SpecConfig {
    fn default() -> Self {
        Self {
            db_min: 0.0,
            db_max: 50.0,
            scale: SpecScale::Linear,
        }
    }
}

pub struct Spectrogram {
    tex: TextureHandle,
    img: ColorImage,
    tex_log: TextureHandle,
    img_log: ColorImage,
    sample_rx: Receiver<Vec<i16>>,
    state: AnalysisState,
    buffer: VecDeque<i16>,
}

const IMG_WIDTH: usize = 512;
const IDX_MIN: usize = fft::hz_to_idx(50.0);
const IDX_MAX: usize = fft::hz_to_idx(10_000.0);

impl SpecConfig {
    pub fn ui(&mut self, ui: &mut Ui) {
        ui.heading("Spectrogram");
        egui::Grid::new("spec_grid")
            .num_columns(2)
            .spacing([40.0, 4.0])
            .striped(true)
            .show(ui, |ui| {
                let min_sep = 10.0;

                ui.label("Db min");
                let min = ui.add(Slider::new(&mut self.db_min, -100.0..=0.0));
                ui.end_row();
                ui.label("Db max");
                let max = ui.add(Slider::new(&mut self.db_max, -50.0..=100.0));
                ui.end_row();

                if min.changed() {
                    if self.db_max - self.db_min < min_sep {
                        self.db_max = self.db_min + min_sep;
                    }
                } else if max.changed() {
                    if self.db_max - self.db_min < min_sep {
                        self.db_min = self.db_max - min_sep;
                    }
                }

                ui.label("Scale");
                ComboBox::new("spec_scale", "")
                    .selected_text(match self.scale {
                        SpecScale::Linear => "linear",
                        SpecScale::Log => "log",
                    })
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut self.scale, SpecScale::Linear, "linear");
                        ui.selectable_value(&mut self.scale, SpecScale::Log, "log");
                    });
                ui.end_row();
            });
        ui.label(format!("Image size: {IMG_WIDTH}x{}", IDX_MAX - IDX_MIN));
    }
}

fn shift_img(img: &mut ColorImage, mut f: impl FnMut(usize) -> Color32) {
    // rotating left shifts stuff left; we are overwriting the last column anyway
    //
    // 0 1 2     1 2 |3
    // 3 4 5  -> 4 5 |6
    // 6 7 8     7 8 |0
    img.pixels.rotate_left(1);

    // now write data in the last column
    let [w, h] = img.size;
    for (i, p_idx) in (0..h).into_iter().map(|y| (h - y) * w - 1).enumerate() {
        img.pixels[p_idx] = f(i);
    }
}

impl Spectrogram {
    pub fn new(ctx: &Context, sample_rx: Receiver<Vec<i16>>) -> Self {
        let img = ColorImage::new([IMG_WIDTH, IDX_MAX - IDX_MIN], Color32::BLACK);
        let img_log = ColorImage::new([IMG_WIDTH, IDX_MAX - IDX_MIN], Color32::BLACK);
        Self {
            tex: ctx.load_texture("spectrogram", img.clone(), TextureOptions::NEAREST),
            img,
            tex_log: ctx.load_texture("spectrogram_log", img_log.clone(), TextureOptions::NEAREST),
            img_log,
            sample_rx,
            state: Default::default(),
            buffer: VecDeque::from_iter(repeat(0).take(SAMPLE_SIZE)),
        }
    }

    pub fn ui(&mut self, ui: &mut Ui, cfg: &SpecConfig) {
        let mut any = false;
        while let Ok(samples) = self.sample_rx.try_recv() {
            let window_len = SAMPLE_SIZE / SAMPLE_WINDOWS;
            assert_eq!(samples.len(), window_len);
            self.buffer.drain(0..window_len);
            self.buffer.extend(&samples);

            self.state = AnalysisState::from_prev(&self.state, self.buffer.iter().cloned());
            let spec = &self.state.fft_out.db;
            shift_img(&mut self.img, |i| {
                let db = *spec[IDX_MIN + i];
                cmap::magma_cmap((db - cfg.db_min) / (cfg.db_max - cfg.db_min))
            });
            let [_, h] = self.img_log.size;
            shift_img(&mut self.img_log, |i| {
                let (h, i) = (h as f32, i as f32);

                let idx_lo = scaling_log(i, h);
                let idx_hi = scaling_log(i + 1.0, h);
                let idx_hi = f32::min(idx_hi, h - 1.);
                let idx_lo = f32::min(idx_hi, idx_lo);

                let db = if idx_lo.floor() != idx_hi.floor() {
                    let lo_frac = idx_lo.ceil() - idx_lo;
                    let hi_frac = idx_hi - idx_hi.floor();

                    let range = &spec[idx_lo as usize..=idx_hi as usize];
                    let mut acc = 0.0;
                    acc += *range[0] * lo_frac;
                    acc += *range[range.len() - 1] * hi_frac;
                    acc += range[1..range.len() - 1].iter().map(|d| **d).sum::<f32>();

                    acc / (idx_hi - idx_lo)
                } else {
                    *spec[idx_lo as usize]
                };
                cmap::magma_cmap((db - cfg.db_min) / (cfg.db_max - cfg.db_min))
            });

            any = true;
        }
        if any {
            self.tex.set(self.img.clone(), TextureOptions::NEAREST);
            self.tex_log
                .set(self.img_log.clone(), TextureOptions::NEAREST);
        }

        let active_tex = match cfg.scale {
            SpecScale::Linear => &self.tex,
            SpecScale::Log => &self.tex_log,
        };

        ui.add(
            Image::new(active_tex)
                .maintain_aspect_ratio(false)
                .shrink_to_fit(),
        );
    }
}

///     h => h
///     0 => 0
/// scales interval in between with a logarithmic function
fn scaling_log(x: f32, h: f32) -> f32 {
    h * (1.0 - (h + 1.0 - x).log(h + 1.0))
}
