use std::{
    collections::VecDeque,
    fs::File,
    io::BufWriter,
    iter::repeat,
    sync::mpsc::{Receiver, Sender},
};

use egui::{ColorImage, Context, Image, Slider, TextureHandle, Ui};

use lib::{
    cfg::AnalysisConfig,
    state::{AnalysisState, AudibleSpec},
    unit,
};
use serde::{Deserialize, Serialize};
use strum::{Display, EnumIter};

use crate::{
    app::AppState,
    audio,
    util::{self, ShiftImage},
};

mod cmap;
mod graph;

#[derive(Serialize, Deserialize)]
#[serde(default)]
pub struct SpecConfig {
    db_min: f32,
    db_max: f32,
    scale: SpecScale,
    data: SpecData,
    pub power: graph::GraphState,
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
    PercussiveFiltered,
}

impl Default for SpecConfig {
    fn default() -> Self {
        Self {
            db_min: 0.0,
            db_max: 50.0,
            scale: SpecScale::Linear,
            data: SpecData::Normal,
            power: Default::default(),
        }
    }
}

impl SpecConfig {
    pub fn ui(
        &mut self,
        ui: &mut Ui,
        cfg: &mut AnalysisConfig,
        audio: &mut audio::Audio,
        spec: &Spectrogram,
    ) {
        ui.heading("Spectrogram");
        egui::Grid::new("spec_grid")
            .num_columns(2)
            .spacing([40.0, 4.0])
            .striped(true)
            .show(ui, |ui| {
                let min_sep = 10.0;

                ui.label("Db min");
                let min = ui.add(Slider::new(&mut self.db_min, -20.0..=50.0));
                ui.end_row();
                ui.label("Db max");
                let max = ui.add(Slider::new(&mut self.db_max, -0.0..=100.0));
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

        if ui.button("Save Spectrogram as .png").clicked() {
            save_png("plot/spectrogram-linear.png",  spec.spec.linear.img.img());
            save_png("plot/spectrogram-log.png",  spec.spec.log.img.img());
            std::process::Command::new("python3")
                .arg("plot/plot.py")
                .spawn()
                .expect("Failed to run python3 plot/plot.py");
        }
    }
}

fn save_png(path: &str, img: &ColorImage) {
    let file = File::create(path).unwrap();
    let ref mut w = BufWriter::new(file);
    let mut encoder = png::Encoder::new(w, img.width() as u32, img.height() as u32);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);

    let mut writer = encoder.write_header().unwrap();
    writer.write_image_data(img.as_raw()).unwrap();
}

struct SpectrogramImage {
    img: ShiftImage,
    scale: fn(f32) -> f32,
}

impl SpectrogramImage {
    fn new(ctx: &egui::Context, name: &str, size: [usize; 2], scale: fn(f32) -> f32) -> Self {
        Self {
            img: ShiftImage::new(ctx, name, size),
            scale,
        }
    }

    fn update_from_db(&mut self, spec: &AudibleSpec<unit::Db>, cfg: &SpecConfig) {
        let [_, h] = self.img.size();
        let scale = self.scale;
        self.img.shift_img(|i| {
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
            SpecScale::Linear => self.linear.img.tex(),
            SpecScale::Log => self.log.img.tex(),
        }
    }
}

pub struct Spectrogram {
    spec: SpectrogramImageSet,
    sample_rx: Receiver<Vec<i16>>,
    audio_tx: Sender<Vec<i16>>,
    pub state: AnalysisState,
    pub buffer: VecDeque<i16>,
    pub hps_energy: graph::Graph,
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
            hps_energy: graph::Graph::new(512),
        }
    }
}

pub fn ui(ui: &mut Ui, state: &mut AppState) {
    let spec = &mut state.spectrogram;
    while let Ok(samples) = spec.sample_rx.try_recv() {
        let hop_len = state.cfg.fft.hop_len;
        assert_eq!(samples.len(), hop_len);

        spec.buffer.drain(0..hop_len);
        spec.buffer.extend(&samples);
        spec.audio_tx
            .send(state.playback.audio_samples(
                &state.persistent.audio,
                samples,
                &state.cfg,
                &spec.state,
            ))
            .unwrap();

        take_mut::take_or_recover(
            &mut spec.state,
            || AnalysisState::blank(&state.cfg),
            |s| AnalysisState::from_prev(&state.cfg, s, spec.buffer.iter().cloned()),
        );

        let data = match state.persistent.spec_cfg.data {
            SpecData::Normal => &spec.state.fft.db,
            SpecData::HarmonicallyEnhanced => &spec.state.hps.h_enhanced.into_db(),
            SpecData::PercussivelyEnhanced => &spec.state.hps.p_enhanced.into_db(),
            SpecData::Harmonic => &spec.state.hps.harmonic.into_db(),
            SpecData::Residual => &spec.state.hps.residual.into_db(),
            SpecData::Percussive => &spec.state.hps.percussive.into_db(),
            SpecData::PercussiveFiltered => &spec.state.power.p_filtered.into_db(),
        };

        spec.spec.update_from_db(data, &state.persistent.spec_cfg);
        spec.hps_energy.update(&spec.state);
    }

    ui.add(
        Image::new(&spec.spec.tex(state.persistent.spec_cfg.scale))
            .maintain_aspect_ratio(false)
            .shrink_to_fit(),
    );
}
