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

pub enum DataVec<'a, T, U> {
    Owned(VecDeque<T>),
    Derived {
        vec: &'a VecDeque<U>,
        func: Box<dyn Fn(&U) -> T + 'a>,
    },
}

impl<'a, T, U> DataVec<'a, T, U> {
    pub fn new(size: usize) -> Self {
        Self::Owned(VecDeque::with_capacity(size))
    }

    pub fn push(&mut self, val: T) {
        match self {
            Self::Owned(vec) => {
                if vec.len() == vec.capacity() {
                    vec.pop_front();
                }
                vec.push_back(val);
            }
            Self::Derived { .. } => panic!("Cannot push to a Derived DataVec"),
        }
    }

    pub fn derive<F: Fn(&T) -> V + 'a, V: Into<f64>>(&'a self, func: F) -> DataVec<'a, f64, T> {
        match self {
            Self::Owned(vec) => DataVec::Derived {
                vec,
                func: Box::new(move |v| func(v).into()),
            },
            Self::Derived { .. } => panic!(
                "Its probably possible to derive twice with some type fuckery im just too lazy"
            ),
        }
    }
}

impl<'a, T: Into<f64>, U> DataVec<'a, T, U> {
    pub fn plot_points(&self) -> PlotPoints
    where
        T: ToOwned<Owned = T>,
    {
        match self {
            Self::Owned(vec) => vec
                .iter()
                .enumerate()
                .map(|(i, v)| [(vec.capacity() - vec.len() + i) as f64, v.to_owned().into()])
                .collect(),
            Self::Derived { vec, func } => vec
                .iter()
                .map(|v| func(v))
                .enumerate()
                .map(|(i, v)| [(vec.capacity() - vec.len() + i) as f64, v.to_owned().into()])
                .collect(),
        }
    }

    pub fn line(&self) -> Line
    where
        T: ToOwned<Owned = T>,
    {
        Line::new(self.plot_points())
    }
}
