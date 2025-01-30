use std::{fs, sync::mpsc::channel};

use lib::{cfg::AnalysisConfig, state::AnalysisContext};

use crate::{audio, spectrogram};

#[derive(serde::Deserialize, serde::Serialize, Default)]
#[serde(default)]
pub struct PersistentAppState {
    pub audio: audio::Audio,
    pub spec_cfg: spectrogram::SpecConfig,
}

pub struct AppState {
    pub persistent: PersistentAppState,
    pub ctx: AnalysisContext,
    pub playback: audio::Playback,
    pub spectrogram: spectrogram::Spectrogram,
}

impl AppState {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let mut persistent =
            eframe::get_value::<PersistentAppState>(cc.storage.unwrap(), eframe::APP_KEY)
                .unwrap_or_default();
        let (sample_tx, sample_rx) = channel();
        let cfg = fs::read_to_string("config.toml")
            .ok()
            .and_then(|s| toml::from_str::<AnalysisConfig>(&s).ok())
            .unwrap_or_default();
        Self {
            playback: audio::Playback::new(&mut persistent.audio, sample_tx),
            spectrogram: spectrogram::Spectrogram::new(&cc.egui_ctx, &cfg, sample_rx),
            persistent,
            ctx: AnalysisContext::new(cfg),
        }
    }
}

impl eframe::App for AppState {
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value::<PersistentAppState>(storage, eframe::APP_KEY, &self.persistent);
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::SidePanel::left("Configuration").show(ctx, |ui| {
            audio::ui(ui, &self.ctx.cfg, &mut self.persistent.audio, &mut self.playback);
            ui.separator();
            self.persistent.spec_cfg.ui(ui, &self.ctx);
            ui.separator();
            let export = ui.button("Export config to `config.toml`");
            if export.clicked() {
                fs::write("config.toml", toml::to_string(&self.ctx.cfg).unwrap()).unwrap();
            }
        });

        egui::CentralPanel::default()
            .show(ctx, |ui| self.spectrogram.ui(ui, &self.ctx, &self.persistent.spec_cfg));

        if self.persistent.audio.playing {
            ctx.request_repaint();
        }
    }
}
