use std::{
    collections::VecDeque,
    iter::repeat,
    sync::mpsc::{Receiver, Sender},
};

use egui::{Color32, ColorImage, Context, Image, Slider, TextureHandle, TextureOptions, Ui};

use lib::{
    cfg::AnalysisConfig,
    state::{AnalysisState, AudibleSpec},
    unit,
};
use serde::{Deserialize, Serialize};
use strum::{Display, EnumIter};

use crate::{audio, cmap, util};

#[derive(Serialize, Deserialize)]
#[serde(default)]
pub struct SpecConfig {
    db_min: f32,
    db_max: f32,
    scale: SpecScale,
    data: SpecData,
}

#[derive(Serialize, Deserialize, PartialEq, Clone, Copy, EnumIter, Display)]
pub enum SpecScale {
    Linear,
    Log,
}

#[derive(Serialize, Deserialize, PartialEq, Clone, Copy, EnumIter, Display)]
pub enum SpecData {
    Normal,
    HarmonicallyEnhanced,
    PercussivelyEnhanced,
    Harmonic,
    Residual,
    Percussive,
}

impl Default for SpecConfig {
    fn default() -> Self {
        Self {
            db_min: 0.0,
            db_max: 50.0,
            scale: SpecScale::Linear,
            data: SpecData::Normal,
        }
    }
}

impl SpecConfig {
    pub fn ui(&mut self, ui: &mut Ui, cfg: &mut AnalysisConfig, audio: &mut audio::Audio) {
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
                util::enum_combobox(ui, "spec_scale", "", &mut self.scale);
                ui.end_row();

                ui.label("Input");
                let res = util::enum_combobox(ui, "spec_data", "", &mut self.data);
                ui.end_row();
                
                if let Some(v) = res.inner {
                    if v[3..=5].iter().any(|v| v.changed()) {
                        audio.harmonic = v[3].changed();
                        audio.residual = v[4].changed();
                        audio.percussive = v[5].changed();
                    }
                }

                ui.label("βh");
                ui.add(Slider::new(&mut cfg.hps.h_factor, 1.0..=10.0));
                ui.end_row();
                
                ui.label("βp");
                ui.add(Slider::new(&mut cfg.hps.p_factor, 1.0..=10.0));
                ui.end_row();
            });
        ui.label(format!(
            "Frequency Range: {}, {}",
            cfg.spectrogram.min_frequency, cfg.spectrogram.max_frequency
        ));
        ui.label(format!("Indeces: {}", cfg.max_idx() - cfg.min_idx()));
    }
}

struct SpectrogramImage {
    tex: TextureHandle,
    img: ColorImage,
    dirty: bool,
    scale: fn(f32) -> f32,
}

impl SpectrogramImage {
    fn new(ctx: &egui::Context, name: &str, size: [usize; 2], scale: fn(f32) -> f32) -> Self {
        let img = ColorImage::new(size, Color32::BLACK);
        Self {
            tex: ctx.load_texture(name, img.clone(), TextureOptions::LINEAR),
            img,
            dirty: false,
            scale,
        }
    }

    fn size(&self) -> [usize; 2] {
        self.img.size
    }

    fn shift_img(&mut self, mut f: impl FnMut(usize) -> Color32) {
        // rotating left shifts stuff left; we are overwriting the last column anyway
        //
        // 0 1 2     1 2 |3
        // 3 4 5  -> 4 5 |6
        // 6 7 8     7 8 |0
        self.img.pixels.rotate_left(1);

        // now write data in the last column
        let [w, h] = self.size();
        for (i, p_idx) in (0..h).into_iter().map(|y| (h - y) * w - 1).enumerate() {
            self.img.pixels[p_idx] = f(i);
        }
        self.dirty = true;
    }

    fn update_from_db(&mut self, spec: &AudibleSpec<unit::Db>, cfg: &SpecConfig) {
        let [_, h] = self.size();
        let scale = self.scale;
        self.shift_img(|i| {
            let (len, h, i) = (spec.len() as f32, h as f32, i as f32);

            let idx_lo = (scale)(i / h) * len;
            let idx_hi = (scale)((i + 1.0) / h) * len;
            let idx_hi = f32::min(idx_hi, len - 1.0);
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
    }

    fn tex(&mut self) -> TextureHandle {
        if self.dirty {
            self.tex.set(self.img.clone(), TextureOptions::LINEAR);
            self.dirty = false;
        }
        self.tex.clone()
    }
}

struct SpectrogramImageSet {
    linear: SpectrogramImage,
    log: SpectrogramImage,
}

impl SpectrogramImageSet {
    fn scale_img_size(scale: SpecScale) -> [usize; 2] {
        match scale {
            SpecScale::Linear => [512, 512],
            SpecScale::Log => [512, 512],
        }
    }

    fn new(ctx: &egui::Context, name: &str) -> Self {
        const B: f32 = 1024.0;
        Self {
            linear: SpectrogramImage::new(
                ctx,
                &format!("{name}_linear"),
                Self::scale_img_size(SpecScale::Linear),
                |x| x,
            ),
            log: SpectrogramImage::new(
                ctx,
                &format!("{name}_log"),
                Self::scale_img_size(SpecScale::Log),
                |x| 1.0 - (B - B * x + x).log(B),
            ),
        }
    }

    fn update_from_db(&mut self, db: &AudibleSpec<unit::Db>, cfg: &SpecConfig) {
        self.linear.update_from_db(db, cfg);
        self.log.update_from_db(db, cfg);
    }

    fn tex(&mut self, scale: SpecScale) -> TextureHandle {
        match scale {
            SpecScale::Linear => self.linear.tex(),
            SpecScale::Log => self.log.tex(),
        }
    }
}

pub struct Spectrogram {
    spec: SpectrogramImageSet,
    sample_rx: Receiver<Vec<i16>>,
    audio_tx: Sender<Vec<i16>>,
    pub state: AnalysisState,
    pub buffer: VecDeque<i16>,
}

impl Spectrogram {
    pub fn new(
        ctx: &Context,
        cfg: &AnalysisConfig,
        sample_rx: Receiver<Vec<i16>>,
        audio_tx: Sender<Vec<i16>>,
    ) -> Self {
        // let img = ColorImage::new([IMG_WIDTH, IDX_MAX], Color32::BLACK);
        // let img_log = ColorImage::new([IMG_WIDTH, IDX_MAX], Color32::BLACK);
        Self {
            // tex: ctx.load_texture("spectrogram", img.clone(), TextureOptions::NEAREST),
            // img,
            // tex_log: ctx.load_texture("spectrogram_log", img_log.clone(), TextureOptions::NEAREST),
            // img_log,
            spec: SpectrogramImageSet::new(ctx, "spectrogram"),
            sample_rx,
            audio_tx,
            state: AnalysisState::blank(cfg),
            buffer: VecDeque::from_iter(repeat(0).take(cfg.fft.frame_len)),
        }
    }

    pub fn ui(
        &mut self,
        ui: &mut Ui,
        cfg: &AnalysisConfig,
        scfg: &SpecConfig,
        audio: &audio::Audio,
        playback: &mut audio::Playback,
    ) {
        while let Ok(samples) = self.sample_rx.try_recv() {
            let hop_len = cfg.fft.hop_len;
            assert_eq!(samples.len(), hop_len);

            self.buffer.drain(0..hop_len);
            self.buffer.extend(&samples);
            self.audio_tx
                .send(playback.audio_samples(audio, samples, cfg, &self.state))
                .unwrap();

            take_mut::take_or_recover(
                &mut self.state,
                || AnalysisState::blank(cfg),
                |s| AnalysisState::from_prev(cfg, s, self.buffer.iter().cloned()),
            );

            let data = match scfg.data {
                SpecData::Normal => &self.state.fft.db,
                SpecData::HarmonicallyEnhanced => &self.state.hps.h_enhanced.into_db(),
                SpecData::PercussivelyEnhanced => &self.state.hps.p_enhanced.into_db(),
                SpecData::Harmonic => &self.state.hps.harmonic.into_db(),
                SpecData::Residual => &self.state.hps.residual.into_db(),
                SpecData::Percussive => &self.state.hps.percussive.into_db(),
            };

            self.spec.update_from_db(data, scfg);
        }

        ui.add(
            Image::new(&self.spec.tex(scfg.scale))
                .maintain_aspect_ratio(false)
                .shrink_to_fit(),
        );
    }
}
