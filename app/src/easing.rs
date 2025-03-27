use egui::{Color32, Pos2};
use egui_plot::{Line, MarkerShape, PlotPoint, PlotUi, Points};
use fields_iter::{FieldsIter, FieldsIterMut};
use lib::{
    Vec2,
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
            selected: FieldsIter::new(ease).next().map(|(n, v)| n).unwrap(),
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
                let mut drawn = Vec2::default();
                let mut d = false;
                for t in 0..=100 {
                    let t = t as f32 / 100.0;
                    let res = ease.variant.parametric(t);
                    points.push([res.x as f64, res.y as f64]);

                    if !d && t > ease.last_x {
                        drawn = res;
                        d = true;
                    }
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
                ui.points(
                    Points::new([drawn.x as f64, drawn.y as f64])
                        .shape(MarkerShape::Cross)
                        .radius(5.0),
                );
            });
        ui.label("Min");
        float_edit_field(ui, &mut ease.min);
        ui.label("Max");
        float_edit_field(ui, &mut ease.max);
    }
}

fn float_edit_field(ui: &mut egui::Ui, value: &mut f32) -> egui::Response {
    let mut tmp_value = format!("{}", value);
    let res = ui.text_edit_singleline(&mut tmp_value);
    if let Ok(result) = tmp_value.parse() {
        *value = result;
    }
    res
}
