use std::sync::mpsc::Receiver;

use egui::{Color32, ColorImage, Context, Image, Slider, TextureHandle, TextureOptions, Ui};

use lib::{SAMPLE_SIZE, fft, state::AnalysisState};
use serde::{Deserialize, Serialize};

use crate::cmap;

#[derive(Serialize, Deserialize)]
pub struct SpecConfig {
    db_min: f32,
    db_max: f32,
}

impl Default for SpecConfig {
    fn default() -> Self {
        Self {
            db_min: -70.0,
            db_max: 50.0,
        }
    }
}

pub struct Spectrogram {
    tex: TextureHandle,
    img: ColorImage,
    sample_rx: Receiver<Vec<i16>>,
    state: AnalysisState,
    samples: Vec<i16>,
}

const IMG_WIDTH: usize = 512;
const DFT_IDX: usize = fft::hz_to_idx(10_000.0);

const HOP_LENGTH: usize = 4;

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
            });
        ui.label(format!("Image size: {IMG_WIDTH}x{DFT_IDX}"));
    }
}

impl Spectrogram {
    pub fn new(ctx: &Context, sample_rx: Receiver<Vec<i16>>) -> Self {
        let img = ColorImage::new([IMG_WIDTH, DFT_IDX], Color32::BLACK);
        Self {
            tex: ctx.load_texture("spectrogram", img.clone(), TextureOptions::LINEAR),
            img,
            sample_rx,
            state: Default::default(),
            samples: vec![0; SAMPLE_SIZE],
        }
    }

    pub fn ui(&mut self, ui: &mut Ui, cfg: &SpecConfig) {
        let mut any = false;
        while let Ok(samples) = self.sample_rx.try_recv() {
            let combined = self
                .samples
                .iter()
                .cloned()
                .chain(samples.iter().cloned())
                .collect::<Vec<_>>();
            assert_eq!(combined.len(), SAMPLE_SIZE * 2);

            // rotating left shifts stuff left; we are overwriting the last columns anyway
            //
            // 0 1 2     1 2 |3
            // 3 4 5  -> 4 5 |6
            // 6 7 8     7 8 |0
            self.img.pixels.rotate_left(HOP_LENGTH);

            for i in 0..HOP_LENGTH {
                let n0 = SAMPLE_SIZE / HOP_LENGTH * i;
                self.state = AnalysisState::from_prev(&self.state, &combined[n0..n0 + SAMPLE_SIZE]);

                // now write fft data in the last column
                let [w, h] = self.img.size;
                for (i, p_idx) in (0..h)
                    .into_iter()
                    .map(|y| (h - y) * w - HOP_LENGTH + i)
                    .enumerate()
                {
                    let db = *self.state.fft_out.db[i];
                    self.img.pixels[p_idx] =
                        cmap::magma_cmap((db - cfg.db_min) / (cfg.db_max - cfg.db_min));
                }
            }
            self.samples = samples;

            any = true;
        }
        if any {
            self.tex.set(self.img.clone(), TextureOptions::LINEAR);
        }
        ui.add(
            Image::new(&self.tex)
                .maintain_aspect_ratio(false)
                .shrink_to_fit(),
        );
    }
}
