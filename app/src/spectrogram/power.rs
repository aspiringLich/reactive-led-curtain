use egui::{Color32, Vec2, Window};
use egui_plot::{Legend, Line, Plot, PlotPoints, Points};
use lib::state::AnalysisState;
use median::Filter;

use std::collections::VecDeque;
use std::iter;

use crate::util::{DataVec, uninteractable_plot};

pub struct HpsPower {
    h_raw: DataVec<f32>,
    r_raw: DataVec<f32>,
    p_raw: DataVec<f32>,
    len: usize,
    tab: Tab,
}

enum Tab {
    Hrp,
    Percussive,
}

impl HpsPower {
    pub fn new(len: usize) -> Self {
        Self {
            h_raw: DataVec::new(len),
            r_raw: DataVec::new(len),
            p_raw: DataVec::new(len),
            len,
            tab: Tab::Hrp,
        }
    }

    pub fn ui(&mut self, ctx: &egui::Context) {
        Window::new("Power").show(ctx, |ui| {
            ui.horizontal(|ui| {
                if ui.button("HPS Power").clicked() {
                    self.tab = Tab::Hrp;
                }
                if ui.button("Percussive").clicked() {
                    self.tab = Tab::Percussive;
                }
            });
            ui.separator();
            match self.tab {
                Tab::Hrp => self
                    .default_plot("hrp")
                    .include_y(300.0)
                    .show(ui, |plot_ui| {
                        plot_ui.line(
                            self.p_raw.line()
                                .name("Percussive")
                                .color(Color32::YELLOW),
                        );
                        plot_ui.line(
                            self.r_raw.line()
                                .name("Residual")
                                .color(Color32::GREEN),
                        );
                        plot_ui.line(
                            self.h_raw.line()
                                .name("Harmonic")
                                .color(Color32::RED),
                        );
                    }),
                Tab::Percussive => {
                    self.default_plot("percussive")
                        .include_y(300.0)
                        .show(ui, |plot_ui| {
                            plot_ui.line(
                                self.p_raw.line()
                                    .name("Percussive Raw")
                                    .color(Color32::YELLOW),
                            );
                        })
                }
            }
        });
    }

    pub fn update(&mut self, state: &AnalysisState) {
        self.h_raw.push(state.power.h_power_raw);
        self.r_raw.push(state.power.r_power_raw);
        self.p_raw.push(state.power.p_power_raw);
    }

    fn default_plot<'a>(&self, id: impl std::hash::Hash) -> Plot<'a> {
        uninteractable_plot(id)
            .legend(Default::default())
            .include_x(0.0)
            .include_x(self.len as f32)
            .include_y(0.0)
            .set_margin_fraction(Vec2::ZERO)
    }
}
