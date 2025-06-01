use std::{
    borrow::Cow,
    fs::File,
    io::BufWriter,
    sync::mpsc::{Receiver, Sender},
    time::Duration,
};

use egui::{ColorImage, Context, Image, Slider, TextureHandle, Ui};

use lib::{
    cfg::AnalysisConfig,
    ebur128::EbuR128,
    state::{AnalysisState, AudibleSpec},
    unit,
};
use puffin_egui::puffin;
use rodio::Source;
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
    HarmonicFiltered,
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
        playback: &mut audio::Playback,
        audio: &mut audio::Audio,
        spec: &mut Spectrogram,
    ) {
        puffin::profile_function!();
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

        if ui.button("Regenerate Spectrogram").clicked()
            && let Some(decoder) = &mut playback.decoder
        {
            spec.spec.reset();
            let mut state = AnalysisState::blank(cfg);
            let time = decoder.get_pos();
            let hop_duration =
                Duration::from_secs_f32(cfg.fft.hop_len as f32 / cfg.fft.sample_rate as f32);

            decoder
                .try_seek(
                    time.checked_sub(hop_duration * (cfg.spectrogram.time_width) as u32)
                        .unwrap_or_default(),
                )
                .unwrap();

            while decoder.get_pos()
                < time
                    .checked_sub(Duration::from_secs_f32(0.001))
                    .unwrap_or_default()
            {
                let hop = decoder.take(cfg.fft.hop_len).collect::<Vec<_>>();
                state = AnalysisState::from_prev(cfg, state, hop.into_iter(), &mut spec.ebur);
                spec.spec.update_from_db(&specdata(self.data, &state), self);
            }
            decoder.try_seek(time).unwrap();
        }
        if ui.button("Save Spectrogram as .png").clicked() {
            save_png("plot/spectrogram-linear.png", spec.spec.linear.img.img());
            save_png("plot/spectrogram-log.png", spec.spec.log.img.img());
            std::process::Command::new("python3")
                .arg("plot/plot.py")
                .spawn()
                .expect("Failed to run python3 plot/plot.py");
            log::info!("Saved images at plot/spectrogram-*.png");
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
    pub img: ShiftImage,
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

    pub fn reset(&mut self) {
        self.linear.img.reset();
        self.log.img.reset();
    }
}

pub struct Spectrogram {
    spec: SpectrogramImageSet,
    sample_rx: Receiver<Vec<i16>>,
    audio_tx: Sender<Vec<i16>>,
    pub state: AnalysisState,
    pub hps_energy: graph::Graph,
    pub ebur: EbuR128,
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
            hps_energy: graph::Graph::new(512),
            ebur: cfg.ebur(),
        }
    }
}

pub fn ui(ui: &mut Ui, state: &mut AppState, cfg: &AnalysisConfig) {
    puffin::profile_function!();
    let spec = &mut state.spectrogram;

    let mut max_iter = 4;
    while let Ok(samples) = spec.sample_rx.try_recv() {
        max_iter -= 1;
        if max_iter < 0 {
            break;
        }

        let hop_len = cfg.fft.hop_len;
        assert_eq!(samples.len(), hop_len);

        spec.audio_tx
            .send(state.playback.audio_samples(
                &state.persistent.audio,
                samples.clone(),
                cfg,
                &spec.state,
            ))
            .unwrap();

        take_mut::take_or_recover(
            &mut spec.state,
            || AnalysisState::blank(cfg),
            |s| AnalysisState::from_prev(cfg, s, samples.iter().cloned(), &mut spec.ebur),
        );

        spec.spec.update_from_db(
            &specdata(state.persistent.spec_cfg.data, &spec.state),
            &state.persistent.spec_cfg,
        );
        spec.hps_energy.update(&spec.state);
    }

    if let Some(port) = state.port.as_mut() {
        puffin::profile_scope!("serialport write");
        
        let img = state.light.img();
        for col in 0..img.width() {
            let mut data = vec![0; img.height() * 3 + 1];
            data[0] = col as u8;
            for row in 0..img.height() {
                let index = (row * 3 + 1) as usize;
                let pixel = img[(col, row)];
                data[index] = pixel.g();
                data[index + 1] = pixel.r();
                data[index + 2] = pixel.b();
            }

            // cobs encode
            let mut encoded = cobs::encode_vec(&data);
            encoded.push(0);

            let res = port.write(&encoded);
            // dbg!(encoded);
            if res.is_err() {
                log::error!("Failed to write data to port: {:?}", res);
            }
        }
    }

    ui.add(
        Image::new(&spec.spec.tex(state.persistent.spec_cfg.scale))
            .maintain_aspect_ratio(false)
            .shrink_to_fit(),
    );
}

fn specdata<'a>(data: SpecData, state: &'a AnalysisState) -> Cow<'a, AudibleSpec<unit::Db>> {
    fn o<'a>(x: AudibleSpec<unit::Db>) -> Cow<'a, AudibleSpec<unit::Db>> {
        Cow::Owned(x)
    }
    match data {
        SpecData::Normal => Cow::Borrowed(&state.fft.db),
        SpecData::HarmonicallyEnhanced => o(state.hps.h_enhanced.into_db()),
        SpecData::PercussivelyEnhanced => o(state.hps.p_enhanced.into_db()),
        SpecData::Harmonic => o(state.hps.harmonic.into_db()),
        SpecData::Residual => o(state.hps.residual.into_db()),
        SpecData::Percussive => o(state.hps.percussive.into_db()),
        SpecData::HarmonicFiltered => o(state.hps.h_filtered.into_db()),
        SpecData::PercussiveFiltered => o(state.hps.p_filtered.into_db()),
    }
}
