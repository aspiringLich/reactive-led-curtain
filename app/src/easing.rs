use egui::{Color32, Pos2, Slider, SliderClamping};
use egui_plot::{Line, MarkerShape, PlotUi, Points};
use fields_iter::{FieldsIter, FieldsIterMut};
use lib::{
    Vec2,
    color::Oklch,
    easing::{CubicBezier, EasingFunction, EasingFunctionVariant, EasingFunctions},
};

use crate::util::{struct_combobox, uninteractable_plot};

pub struct EaseEditor {
    pub selected: &'static str,
    pub handle_reserved: u16,
}

fn handle(ui: &mut PlotUi, origin: [f64; 2], handle: &mut lib::Vec2, id: u16, reserved: &mut u16) {
    assert_ne!(id, 0);
    let radius = 5.0;

    let hover_pos = ui.pointer_coordinate().map(|pos| pos.to_pos2());

    let mut hovered = hover_pos
        .filter(|pos| pos.distance(Pos2::new(handle.x, handle.y)) < radius / 50.0)
        .is_some();

    // if unreserved and clicked on, reserve and update handle
    // dbg!(hovered, ui.response().is_pointer_button_down_on(), *reserved);
    if hovered
        && ui.response().is_pointer_button_down_on()
        && *reserved == 0
        && let Some(pos) = ui.pointer_coordinate()
    {
        *handle = lib::Vec2::new(pos.x as f32, pos.y as f32);
        *handle = handle.clamp(Vec2::ZERO, Vec2::splat(1.0));
        *reserved = id;
    }
    // if reserved and mouse button still held, update handle
    if *reserved == id
        && ui.response().is_pointer_button_down_on()
        && let Some(pos) = ui.pointer_coordinate()
    {
        *handle = lib::Vec2::new(pos.x as f32, pos.y as f32);
        *handle = handle.clamp(Vec2::ZERO, Vec2::splat(1.0));
        hovered = true;
    }
    // if reserved and released, unreserve
    if *reserved == id && !ui.response().is_pointer_button_down_on() {
        *reserved = 0;
    }

    let h = [handle.x as f64, handle.y as f64];
    ui.points(
        Points::new(vec![h])
            .shape(MarkerShape::Circle)
            .radius(radius)
            .color(if hovered {
                Color32::WHITE
            } else {
                Color32::GRAY
            }),
    );
    ui.line(
        Line::new(vec![origin, h])
            .color(Color32::GRAY)
            .allow_hover(false),
    );
}

impl EaseEditor {
    pub fn new(ease: &EasingFunctions) -> Self {
        Self {
            selected: FieldsIter::new(ease).next().map(|(n, _)| n).unwrap(),
            handle_reserved: 0,
        }
    }

    pub fn ui(&mut self, ui: &mut egui::Ui, easing: &mut EasingFunctions) {
        struct_combobox(ui, &easing, "easing_combo", "Easing", &mut self.selected);
        let ease = FieldsIterMut::new(easing)
            .find(|(n, _)| *n == self.selected)
            .and_then(|v| v.1.downcast_mut::<EasingFunction>())
            .unwrap();
        uninteractable_plot("easing_plot")
            .view_aspect(1.0)
            .show(ui, |ui| {
                let mut points = Vec::new();
                for t in 0..=100 {
                    let t = t as f32 / 100.0;
                    let res = ease.variant.parametric(t);
                    points.push([res.x as f64, res.y as f64]);
                }
                ui.line(Line::new(points).color(Color32::WHITE));

                use EasingFunctionVariant as E;
                match &mut ease.variant {
                    E::CubicBezier(CubicBezier { p1, p2 }) => {
                        handle(ui, [0.0, 0.0], p1, 1, &mut self.handle_reserved);
                        handle(ui, [1.0, 1.0], p2, 2, &mut self.handle_reserved);
                    }
                }
                // dbg!(ease.last_x);
                let colors: Vec<Color32> = vec![
                    Oklch::LIGHT.red().into(),
                    Oklch::LIGHT.green().into(),
                    Oklch::LIGHT.blue().into(),
                ];
                for (i, &x) in ease.last_x.iter().enumerate() {
                    let y = ease.variant.solve(x);
                    ui.points(
                        Points::new([x as f64, y as f64])
                            .shape(MarkerShape::Plus)
                            .color(colors[i.rem_euclid(colors.len())])
                            .radius(10.0),
                    );
                }
            });
        ui.horizontal(|ui| {
            ui.label("Min");
            ui.add(Slider::new(&mut ease.min, 0.0..=5.0).clamping(SliderClamping::Never));
        });
        ui.horizontal(|ui| {
            ui.label("Max");
            ui.add(Slider::new(&mut ease.max, 0.0..=5.0).clamping(SliderClamping::Never));
        });
    }
}

// fn float_edit_field(ui: &mut egui::Ui, value: &mut f32, s: &mut String) -> egui::Response {
//     let res = ui.text_edit_singleline(s);
//     if let Ok(result) = s.parse() {
//         *value = result;
//     }
//     res
// }
