use egui::{Button, Color32, Slider, Vec2, Window};
use egui_plot::Plot;
use lib::state::AnalysisState;

use crate::util::{DataVec, uninteractable_plot};

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct PowerState {
    tab: Tab,
    scale: f32,
}

impl Default for PowerState {
    fn default() -> Self {
        Self {
            tab: Tab::Hrp,
            scale: 300.0,
        }
    }
}

pub struct Power {
    h_raw: DataVec<f32>,
    r_raw: DataVec<f32>,
    p_raw: DataVec<f32>,
    dp: DataVec<f32>,
    len: usize,
}

#[derive(Serialize, Deserialize, PartialEq, Default)]
enum Tab {
    #[default]
    Hrp,
    Percussive,
}

impl Power {
    pub fn new(len: usize) -> Self {
        Self {
            h_raw: DataVec::new(len),
            r_raw: DataVec::new(len),
            p_raw: DataVec::new(len),
            dp: DataVec::new(len),
            len,
        }
    }

    pub fn ui(&mut self, state: &mut PowerState, ctx: &egui::Context) {
        Window::new("Power").show(ctx, |ui| {
            ui.horizontal(|ui| {
                if ui
                    .add(Button::new("HPS Power").selected(state.tab == Tab::Hrp))
                    .clicked()
                {
                    state.tab = Tab::Hrp;
                }
                if ui
                    .add(Button::new("Percussive").selected(state.tab == Tab::Percussive))
                    .clicked()
                {
                    state.tab = Tab::Percussive;
                }
            });
            ui.separator();
            ui.horizontal(|ui| {
                ui.label("Scale");
                ui.add(Slider::new(&mut state.scale, 0.0..=600.0).trailing_fill(true));
            });
            match state.tab {
                Tab::Hrp => self
                    .default_plot("hrp")
                    .include_y(state.scale)
                    .show(ui, |plot_ui| {
                        plot_ui.line(self.p_raw.line().name("Percussive").color(Color32::YELLOW));
                        plot_ui.line(self.r_raw.line().name("Residual").color(Color32::GREEN));
                        plot_ui.line(self.h_raw.line().name("Harmonic").color(Color32::RED));
                    }),
                Tab::Percussive => self
                    .default_plot("percussive")
                    .include_y(state.scale)
                    .include_y(-state.scale)
                    .show(ui, |plot_ui| {
                        plot_ui.line(
                            self.p_raw
                                .line()
                                .name("Percussive Raw")
                                .color(Color32::YELLOW),
                        );
                        plot_ui.line(self.dp.line().name("Δp").color(Color32::ORANGE));
                    }),
            };
        });
    }

    pub fn update(&mut self, state: &AnalysisState) {
        let p = &state.power;
        self.h_raw.push(p.h_power_raw);
        self.r_raw.push(p.r_power_raw);
        self.p_raw.push(p.p_power_raw);
        self.dp.push(p.dp);
    }

    fn default_plot<'a>(&self, id: impl std::hash::Hash) -> Plot<'a> {
        uninteractable_plot(id)
            .legend(Default::default())
            // .auto_bounds(Vec2b::FALSE)
            .include_x(0.0)
            .include_x(self.len as f32)
            .include_y(0.0)
            .set_margin_fraction(Vec2::new(0.0, 0.1))
    }
}
