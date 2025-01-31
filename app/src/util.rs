use egui::{ComboBox, InnerResponse, Response, Ui, WidgetText};
use strum::IntoEnumIterator;

pub fn enum_combobox<T: IntoEnumIterator + ToString + PartialEq + Copy>(
    ui: &mut Ui,
    id_salt: impl std::hash::Hash,
    label: impl Into<WidgetText>,
    value: &mut T,
) -> InnerResponse<Option<Vec<Response>>> {
    ComboBox::new(id_salt, label)
        .selected_text(value.to_string())
        .show_ui(ui, |ui| {
            T::iter()
                .map(|v| ui.selectable_value(value, v, v.to_string()))
                .collect()
        })
}
