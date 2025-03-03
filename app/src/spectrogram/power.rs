use egui::{Align, Button, Frame, Layout, Slider, TopBottomPanel, Ui, Vec2, Window};
use egui_plot::{Legend, Plot};
use lib::{
    color::Oklch,
    state::{AnalysisState, power::PowerData},
};

use std::convert::Infallible;

use crate::util::{DataVec, uninteractable_plot};

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
#[serde(default)]
pub struct PowerState {
    tab: Tab,
    scale: f32,
    window: bool,
    legend: bool,
}

impl Default for PowerState {
    fn default() -> Self {
        Self {
            tab: Tab::Hrp,
            scale: 300.0,
            window: false,
            legend: true,
        }
    }
}

pub struct Power {
    data: DataVec<'static, PowerData, Infallible>,
    len: usize,
}

#[derive(Serialize, Deserialize, PartialEq, Default)]
enum Tab {
    #[default]
    Hrp,
    Percussive,
    Ratios,
}

impl Power {
    pub fn new(len: usize) -> Self {
        Self {
            data: DataVec::new(len),
            len,
        }
    }

    pub fn ui(&mut self, state: &mut PowerState, ctx: &egui::Context) {
        let mut window = state.window;
        let ui = |ui: &mut Ui| {
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
                if ui
                    .add(Button::new("Ratios").selected(state.tab == Tab::Ratios))
                    .clicked()
                {
                    state.tab = Tab::Ratios;
                }
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if ui
                        .add(Button::new(if window { "↥" } else { "🗗" }))
                        .clicked()
                    {
                        window = !window;
                    }
                })
            });
            ui.separator();
            ui.horizontal(|ui| {
                ui.label("Scale");
                ui.add(Slider::new(&mut state.scale, 0.0..=600.0).trailing_fill(true));
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    ui.checkbox(&mut state.legend, "Legend");
                })
            });
            match state.tab {
                Tab::Hrp => self
                    .default_plot("hrp", state.legend)
                    .include_y(state.scale)
                    .show(ui, |plot_ui| {
                        plot_ui.line(
                            self.data
                                .derive(|d| d.p_power_raw.val)
                                .line()
                                .name("Percussive")
                                .color(Oklch::LIGHT.yellow()),
                        );
                        plot_ui.line(
                            self.data
                                .derive(|d| d.r_power_raw)
                                .line()
                                .name("Residual")
                                .color(Oklch::LIGHT.green()),
                        );
                        plot_ui.line(
                            self.data
                                .derive(|d| d.h_power_raw)
                                .line()
                                .name("Harmonic")
                                .color(Oklch::LIGHT.red()),
                        );
                    }),
                Tab::Percussive => self
                    .default_plot("percussive", state.legend)
                    .include_y(state.scale)
                    .include_y(-state.scale)
                    .show(ui, |plot_ui| {
                        plot_ui.line(
                            self.data
                                .derive(|d| d.p_power_raw.dval)
                                .line()
                                .name("Δp")
                                .color(Oklch::DIM.yellow()),
                        );
                        plot_ui.line(
                            self.data
                                .derive(|d| d.p_filtered_power.dval)
                                .line()
                                .name("Δfiltered")
                                .color(Oklch::DIM.red()),
                        );
                        plot_ui.line(
                            self.data
                                .derive(|d| d.p_bass_power.val)
                                .line()
                                .name("bass")
                                .color(Oklch::LIGHT.green()),
                        );
                        plot_ui.line(
                            self.data
                                .derive(|d| d.p_power_raw.val)
                                .line()
                                .name("Percussive")
                                .color(Oklch::LIGHT.yellow()),
                        );
                        plot_ui.line(
                            self.data
                                .derive(|d| d.p_filtered_power.val)
                                .line()
                                .name("Filtered")
                                .color(Oklch::LIGHT.red()),
                        );
                    }),
                Tab::Ratios => self
                    .default_plot("ratios", state.legend)
                    .show(ui, |plot_ui| {
                        plot_ui.line(
                            self.data
                                .derive(|d| d.ratio_h_p.average())
                                .line()
                                .name("ratio(h, pf)")
                                .color(Oklch::LIGHT.red()),
                        );
                        plot_ui.line(
                            self.data
                                .derive(|d| d.p_bass_power.val / d.p_filtered_power.val)
                                .line()
                                .name("bass")
                                .color(Oklch::LIGHT.green()),
                        )
                    }),
            };
            window
        };
        if state.window {
            state.window = Window::new("Power")
                .show(ctx, ui)
                .and_then(|r| r.inner)
                .unwrap_or(true);
        } else {
            let panel = TopBottomPanel::top("top")
                .frame(Frame::default().inner_margin(Vec2::new(0.0, 5.0)))
                .resizable(true);
            state.window = panel.show(ctx, ui).inner;
        }
    }

    pub fn update(&mut self, state: &AnalysisState) {
        self.data.push(state.power.clone());
    }

    fn default_plot<'a>(&self, id: impl std::hash::Hash, legend: bool) -> Plot<'a> {
        let plot = uninteractable_plot(id)
            // .auto_bounds(Vec2b::FALSE)
            .include_x(0.0)
            .include_x(self.len as f32)
            .include_y(0.0)
            .include_y(1.0)
            .set_margin_fraction(Vec2::new(0.0, 0.1));
        if legend {
            plot.legend(Legend::default())
        } else {
            plot
        }
    }
}
