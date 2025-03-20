use fields_iter::{FieldsInspect, FieldsIter};
use lib::easing::{EasingFunction, EasingFunctions};

use crate::util::struct_combobox;

pub struct EaseEditor {
    pub selected: (&'static str, EasingFunction),
}

impl EaseEditor {
    pub fn new(ease: &EasingFunctions) -> Self {
        Self {
            selected: FieldsIter::new(ease)
                .next()
                .map(|(n, v)| (n, v.downcast_ref::<EasingFunction>().unwrap().clone()))
                .unwrap(),
        }
    }

    pub fn ui(&mut self, ui: &mut egui::Ui, easing: EasingFunctions) {
        struct_combobox(ui, &easing, "easing", "Easing",  &mut self.selected);
    }
}
