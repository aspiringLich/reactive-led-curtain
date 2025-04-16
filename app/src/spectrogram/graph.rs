use egui::{Align, Button, Frame, Layout, Slider, TopBottomPanel, Ui, Vec2, Window};
use egui_plot::{Corner, Legend, Plot};
use lib::{
    color::Oklch,
    state::{AnalysisState, light::LightData, loudness::LoudnessData, power::PowerData},
};
use puffin_egui::puffin;
use strum::{Display, EnumIter, IntoEnumIterator};

use std::convert::Infallible;

use crate::util::{DataVec, uninteractable_plot};

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
#[serde(default)]
pub struct GraphState {
    tab: Tab,
    scale: f32,
    window: bool,
    legend: bool,
}

impl Default for GraphState {
    fn default() -> Self {
        Self {
            tab: Tab::Hrp,
            scale: 300.0,
            window: false,
            legend: true,
        }
    }
}

pub struct Graph {
    pdata: DataVec<'static, PowerData, Infallible>,
    ldata: DataVec<'static, LightData, Infallible>,
    odata: DataVec<'static, LoudnessData, Infallible>,
    len: usize,
}

#[derive(Serialize, Deserialize, PartialEq, Default, EnumIter, Display)]
enum Tab {
    #[default]
    Hrp,
    Percussive,
    Ratios,
    Light,
    Loudness,
    OctavePower,
    Notes,
}

// for OctavePower and Octave
const NOTES: [(Oklch, &str); 12] = [
    (Oklch::LIGHT.red(), "A"),
    (Oklch::LIGHT.orange(), "A#/Bb"),
    (Oklch::LIGHT.yellow(), "B"),
    (Oklch::LIGHT.lime(), "C"),
    (Oklch::LIGHT.green(), "C#/Db"),
    (Oklch::LIGHT.jade(), "D"),
    (Oklch::LIGHT.cyan(), "D#/Eb"),
    (Oklch::LIGHT.sky_blue(), "E"),
    (Oklch::LIGHT.blue(), "F"),
    (Oklch::LIGHT.indigo(), "F#/Gb"),
    (Oklch::LIGHT.purple(), "G"),
    (Oklch::LIGHT.magenta(), "G#/Ab"),
];

impl Graph {
    pub fn new(len: usize) -> Self {
        Self {
            pdata: DataVec::new(len),
            ldata: DataVec::new(len),
            odata: DataVec::new(len),
            len,
        }
    }

    pub fn ui(&mut self, state: &mut GraphState, ctx: &egui::Context) {
        puffin::profile_function!();
        let mut window = state.window;
        let ui = |ui: &mut Ui| {
            ui.horizontal(|ui| {
                for tab in Tab::iter() {
                    if ui
                        .add(Button::new(tab.to_string()).selected(state.tab == tab))
                        .clicked()
                    {
                        state.tab = tab;
                    }
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
                            self.pdata
                                .derive(|d| d.p_power_raw.val)
                                .line()
                                .name("Percussive")
                                .color(Oklch::LIGHT.yellow()),
                        );
                        plot_ui.line(
                            self.pdata
                                .derive(|d| d.r_power_raw)
                                .line()
                                .name("Residual")
                                .color(Oklch::LIGHT.green()),
                        );
                        plot_ui.line(
                            self.pdata
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
                            self.pdata
                                .derive(|d| d.p_power_raw.dval)
                                .line()
                                .name("Δp")
                                .color(Oklch::DIM.yellow()),
                        );
                        plot_ui.line(
                            self.pdata
                                .derive(|d| d.p_filtered_power.dval)
                                .line()
                                .name("Δfiltered")
                                .color(Oklch::DIM.red()),
                        );
                        plot_ui.line(
                            self.pdata
                                .derive(|d| d.p_bass_power.val)
                                .line()
                                .name("bass")
                                .color(Oklch::LIGHT.green()),
                        );
                        plot_ui.line(
                            self.pdata
                                .derive(|d| d.p_power_raw.val)
                                .line()
                                .name("Percussive")
                                .color(Oklch::LIGHT.yellow()),
                        );
                        plot_ui.line(
                            self.pdata
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
                            self.pdata
                                .derive(|d| d.ratio_h_p.average())
                                .line()
                                .name("ratio(h, pf)")
                                .color(Oklch::LIGHT.red()),
                        );
                        plot_ui.line(
                            self.pdata
                                .derive(|d| d.p_bass_power.val / d.p_filtered_power.val)
                                .line()
                                .name("bass")
                                .color(Oklch::LIGHT.green()),
                        )
                    }),
                Tab::Light => self
                    .default_plot("graph", state.legend)
                    .show(ui, |plot_ui| {
                        plot_ui.line(
                            self.ldata
                                .derive(|d| d.percussive.average())
                                .line()
                                .name("Percussive")
                                .color(Oklch::LIGHT.red()),
                        );
                        plot_ui.line(
                            self.ldata
                                .derive(|d| d.bass_percussive.average())
                                .line()
                                .name("Bass")
                                .color(Oklch::LIGHT.green()),
                        );
                    }),
                Tab::Loudness => self
                    .default_plot("loudness", state.legend)
                    .show(ui, |plot_ui| {
                        plot_ui.line(
                            self.odata
                                .derive(|d| d.st)
                                .line()
                                .name("Short Time")
                                .color(Oklch::LIGHT.green()),
                        );
                        plot_ui.line(
                            self.odata
                                .derive(|d| d.m)
                                .line()
                                .name("Momentary")
                                .color(Oklch::LIGHT.red()),
                        );
                    }),
                Tab::OctavePower => {
                    self.default_plot("octave_power", state.legend)
                        .show(ui, |plot_ui| {
                            for (i, (color, name)) in NOTES.iter().enumerate() {
                                plot_ui.line(
                                    self.pdata
                                        .derive(|d| d.octave_power[i])
                                        .line()
                                        .name(name)
                                        .color(color.to_owned()),
                                )
                            }
                        })
                }
                Tab::Notes => self
                    .default_plot("notes", state.legend)
                    .show(ui, |plot_ui| {
                        for (i, (color, name)) in NOTES.iter().enumerate() {
                            plot_ui.line(
                                self.ldata
                                    .derive(|d| d.notes[i].average())
                                    .line()
                                    .name(name)
                                    .color(color.to_owned()),
                            )
                        }
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
                .frame(
                    Frame::default()
                        .inner_margin(Vec2::new(0.0, 5.0))
                        .fill(ctx.style().visuals.panel_fill),
                )
                .resizable(true);
            state.window = panel.show(ctx, ui).inner;
        }
    }

    pub fn update(&mut self, state: &AnalysisState) {
        self.pdata.push(state.power.clone());
        self.ldata.push(state.light.clone());
        self.odata.push(state.loudness.clone());
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
            plot.legend(Legend::default().position(Corner::LeftTop))
        } else {
            plot
        }
    }
}
