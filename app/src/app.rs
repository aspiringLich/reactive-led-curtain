use std::{fs, sync::mpsc::channel};

use egui::{Frame, Layout, ScrollArea};
use lib::cfg::AnalysisConfig;

use crate::{audio, light, spectrogram};

#[derive(serde::Deserialize, serde::Serialize, Default)]
#[serde(default)]
pub struct PersistentAppState {
    pub audio: audio::Audio,
    pub spec_cfg: spectrogram::SpecConfig,
}

pub struct AppState {
    pub persistent: PersistentAppState,
    pub cfg: AnalysisConfig,
    pub playback: audio::Playback,
    pub spectrogram: spectrogram::Spectrogram,
    pub light: light::Light,
}

impl AppState {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let mut persistent =
            eframe::get_value::<PersistentAppState>(cc.storage.unwrap(), eframe::APP_KEY)
                .unwrap_or_default();
        let (sample_tx, sample_rx) = channel();
        let (audio_tx, audio_rx) = channel();
        let cfg = fs::read_to_string("config.toml")
            .ok()
            .and_then(|s| toml::from_str::<AnalysisConfig>(&s).ok())
            .unwrap_or_default();
        Self {
            playback: audio::Playback::new(&mut persistent.audio, sample_tx, audio_rx, &cfg),
            spectrogram: spectrogram::Spectrogram::new(&cc.egui_ctx, &cfg, sample_rx, audio_tx),
            light: light::Light::new(&cc.egui_ctx, &cfg.light),
            persistent,
            cfg,
        }
    }
}

impl eframe::App for AppState {
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value::<PersistentAppState>(storage, eframe::APP_KEY, &self.persistent);
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::SidePanel::left("Configuration").show(ctx, |ui| {
            ui.with_layout(Layout::bottom_up(egui::Align::Min), |ui| {
                self.light.ui(ctx, ui, &self.cfg.light);
                ui.with_layout(Layout::default(), |ui| {
                    ScrollArea::vertical().show(ui, |ui| {
                        audio::ui(ui, &mut self.persistent.audio, &mut self.playback);
                        audio::playback(&self.cfg, &mut self.persistent.audio, &mut self.playback);
                        ui.separator();
                        self.persistent.spec_cfg.ui(
                            ui,
                            &mut self.cfg,
                            &mut self.playback,
                            &mut self.persistent.audio,
                            &mut self.spectrogram,
                        );
                        ui.separator();
                        let export = ui.button("Export config to `config.toml`");
                        if export.clicked() {
                            fs::write("config.toml", toml::to_string(&self.cfg).unwrap()).unwrap();
                        }
                    });
                });
            });
        });

        self.spectrogram
            .hps_energy
            .ui(&mut self.persistent.spec_cfg.power, ctx);

        let panel = egui::CentralPanel::default().frame(Frame::none().inner_margin(0.0));
        panel.show(ctx, |ui| {
            spectrogram::ui(ui, self);
        });

        if self.persistent.audio.playing {
            ctx.request_repaint();
        }
    }
}
