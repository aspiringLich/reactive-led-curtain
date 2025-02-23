use egui::{Color32, Vec2, Window};
use egui_plot::{Legend, Line, PlotPoints, Points};
use median::Filter;

use std::collections::VecDeque;
use std::iter;

use crate::util::uninteractable_plot;

pub struct HpsEnergy {
    data: VecDeque<[f32; 3]>,
    len: usize,
}

impl HpsEnergy {
    pub fn new(len: usize) -> Self {
        Self {
            data: VecDeque::new(),
            len,
        }
    }

    pub fn update(&mut self, h: f32, r: f32, p: f32) {
        if self.data.len() == self.len {
            self.data.pop_front();
        }
        self.data.push_back([h, r, p]);
    }

    pub fn ui(&self, ctx: &egui::Context) {
        Window::new("HPS Energy").show(ctx, |ui| {
            let plot_points = |idx: usize| {
                self.data
                    .iter()
                    .enumerate()
                    .map(|(i, hps)| [(self.len - self.data.len() + i) as f64, hps[idx] as f64])
                    .collect::<PlotPoints>()
            };
            uninteractable_plot("HPS Energy")
                .legend(Default::default())
                .include_x(0.0)
                .include_x(self.len as f32)
                .include_y(0.0)
                .include_y(3.0)
                .set_margin_fraction(Vec2::ZERO)
                .show(ui, |plot_ui| {
                    plot_ui.line(
                        Line::new(plot_points(2))
                            .name("Percussive")
                            .color(Color32::YELLOW),
                    );
                    plot_ui.line(
                        Line::new(plot_points(1))
                            .name("Residual")
                            .color(Color32::GREEN),
                    );
                    plot_ui.line(
                        Line::new(plot_points(0))
                            .name("Harmonic")
                            .color(Color32::RED),
                    );
                })
        });
    }
}
