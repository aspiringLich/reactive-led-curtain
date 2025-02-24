use egui::{
    Color32, ColorImage, ComboBox, InnerResponse, Response, TextureHandle, TextureOptions, Ui,
    WidgetText,
};
use egui_plot::{Line, Plot, PlotPoints};
use strum::IntoEnumIterator;

use std::collections::VecDeque;

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

pub struct ShiftImage {
    tex: TextureHandle,
    img: ColorImage,
    dirty: bool,
}

impl ShiftImage {
    pub fn new(ctx: &egui::Context, name: &str, size: [usize; 2]) -> Self {
        let img = ColorImage::new(size, Color32::BLACK);
        Self {
            tex: ctx.load_texture(name, img.clone(), TextureOptions::LINEAR),
            img,
            dirty: false,
        }
    }

    pub fn size(&self) -> [usize; 2] {
        self.img.size
    }

    pub fn shift_img(&mut self, mut f: impl FnMut(usize) -> Color32) {
        // rotating left shifts stuff left; we are overwriting the last column anyway
        //
        // 0 1 2     1 2 |3
        // 3 4 5  -> 4 5 |6
        // 6 7 8     7 8 |0
        self.img.pixels.rotate_left(1);

        // now write data in the last column
        let [w, h] = self.size();
        for (i, p_idx) in (0..h).into_iter().map(|y| (h - y) * w - 1).enumerate() {
            self.img.pixels[p_idx] = f(i);
        }
        self.dirty = true;
    }

    pub fn tex(&mut self) -> TextureHandle {
        if self.dirty {
            self.tex.set(self.img.clone(), TextureOptions::LINEAR);
            self.dirty = false;
        }
        self.tex.clone()
    }
}

pub fn uninteractable_plot<'a>(id: impl std::hash::Hash) -> Plot<'a> {
    Plot::new(id)
        .allow_drag(false)
        .allow_zoom(false)
        .allow_scroll(false)
        .allow_boxed_zoom(false)
}

pub struct DataVec<T: Into<f64>> {
    pub vec: VecDeque<T>,
}

impl<T: Into<f64>> std::ops::Deref for DataVec<T> {
    type Target = VecDeque<T>;

    fn deref(&self) -> &Self::Target {
        &self.vec
    }
}

impl<T: Into<f64>> DataVec<T> {
    pub fn new(size: usize) -> Self {
        Self {
            vec: VecDeque::with_capacity(size),
        }
    }

    pub fn push(&mut self, val: T) {
        if self.len() == self.capacity() {
            self.vec.pop_front();
        }
        self.vec.push_back(val);
    }

    pub fn plot_points(&self) -> PlotPoints
    where
        T: ToOwned<Owned = T>,
    {
        self.iter()
            .enumerate()
            .map(|(i, v)| {
                [
                    (self.capacity() - self.len() + i) as f64,
                    v.to_owned().into(),
                ]
            })
            .collect()
    }

    pub fn line(&self) -> Line
    where
        T: ToOwned<Owned = T>,
    {
        Line::new(self.plot_points())
    }
}
