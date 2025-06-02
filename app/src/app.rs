use std::{
    fs,
    sync::{Arc, mpsc::channel},
};

use egui::mutex::Mutex;
use egui::{CollapsingHeader, Frame, Key, Layout, ScrollArea};
use lib::cfg::AnalysisConfig;
use puffin_egui::puffin;

use crate::{audio, easing, light, serialport_thread::SerialPortThread, spectrogram};

#[derive(serde::Deserialize, serde::Serialize, Default)]
#[serde(default)]
pub struct PersistentAppState {
    pub audio: audio::Audio,
    pub spec_cfg: spectrogram::SpecConfig,
    #[serde(skip)]
    pub easing: lib::easing::EasingFunctions,
}

pub struct AppState {
    pub persistent: PersistentAppState,
    pub cfg: AnalysisConfig,
    pub playback: audio::Playback,
    pub spectrogram: spectrogram::Spectrogram,
    pub light: light::Light,
    pub ease: easing::EaseEditor,
    pub serial_thread: Option<SerialPortThread>,
    pub debug: bool,
}

impl AppState {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let mut persistent =
            eframe::get_value::<PersistentAppState>(cc.storage.unwrap(), eframe::APP_KEY)
                .unwrap_or_default();
        let (sample_tx, sample_rx) = channel();
        let (audio_tx, audio_rx) = channel();
        let cfg =  fs::read_to_string("config.toml")
            .ok()
            .and_then(|s| toml::from_str::<AnalysisConfig>(&s).ok())
            .unwrap_or_default();

        let spectrogram =
            spectrogram::Spectrogram::new(&cc.egui_ctx, &cfg, sample_rx, audio_tx);
        let playback =
            audio::Playback::new(&mut persistent.audio, sample_tx, audio_rx, &cfg);
        let ease = easing::EaseEditor::new(&spectrogram.state.easing);
        let light = light::Light::new(&cc.egui_ctx, &cfg.light);
        let serial_thread = Some(SerialPortThread::new());

        Self {
            playback,
            ease,
            spectrogram,
            light,
            persistent,
            cfg,
            serial_thread,
            debug: false,
        }
    }
}

impl eframe::App for AppState {
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        self.persistent.easing = self.spectrogram.state.easing.clone();
        eframe::set_value::<PersistentAppState>(storage, eframe::APP_KEY, &self.persistent);
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if ctx.input(|state| state.key_pressed(Key::D) && state.modifiers.shift) {
            self.debug = !self.debug;
        }

        if self.debug {
            self.debug = puffin_egui::profiler_window(ctx);
            puffin::set_scopes_on(true);
            puffin::GlobalProfiler::lock().new_frame();
            puffin::profile_function!();
        }

        egui::SidePanel::left("Configuration").show(ctx, |ui| {
            ui.with_layout(Layout::bottom_up(egui::Align::Min), |ui| {
                self.light
                    .ui(ctx, ui, &self.cfg.light, &self.spectrogram.state.paint);
                ui.with_layout(Layout::default(), |ui| {
                    ScrollArea::vertical().show(ui, |ui| {
                        CollapsingHeader::new("Audio").show(ui, |ui| {
                            audio::ui(
                                ui,
                                &mut self.persistent.audio,
                                &mut self.playback,
                                &mut self.cfg.loudness,
                            );
                            audio::playback(&self.cfg, &mut self.persistent.audio, &mut self.playback);
                        });
                        ui.separator();
                        CollapsingHeader::new("Spectrogram").show(ui, |ui| {
                            self.persistent.spec_cfg.ui(
                                ui,
                                &mut self.cfg,
                                &mut self.playback,
                                &mut self.persistent.audio,
                                &mut self.spectrogram,
                            );
                        });
                        ui.separator();
                        CollapsingHeader::new("Easing").show(ui, |ui| {
                            self.ease.ui(ui, &mut self.spectrogram.state.easing);
                        });

                        let export = ui.button("Export config");
                        if export.clicked() {
                            fs::write("config.toml", toml::to_string(&self.cfg).unwrap()).unwrap();
                            fs::write(
                                "easing.toml",
                                toml::to_string(&self.spectrogram.state.easing).unwrap(),
                            )
                            .unwrap();
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
