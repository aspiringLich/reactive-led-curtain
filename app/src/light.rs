use std::collections::VecDeque;

use egui::{Color32, ColorImage, Context, Image, TextureHandle, TextureOptions, Ui};
use lib::state::{light::LightConfig, paint::PaintData};
use puffin_egui::puffin;

pub struct Light {
    tex: TextureHandle,
    delay_buf: VecDeque<ColorImage>,
    // img: ColorImage,
}

impl Light {
    pub fn new(ctx: &Context, cfg: &LightConfig) -> Self {
        let img = ColorImage::new([cfg.width as usize, cfg.height as usize], Color32::BLACK);
        Self {
            tex: ctx.load_texture("light", img.clone(), TextureOptions::NEAREST),
            delay_buf: VecDeque::from_iter(std::iter::repeat_n(img, cfg.gui_delay as usize + 1)),
            // img,
        }
    }

    pub fn ui(&mut self, _ctx: &Context, ui: &mut Ui, cfg: &LightConfig, paint: &PaintData) {
        puffin::profile_function!();
        let img = ColorImage {
            size: [cfg.width as usize, cfg.height as usize],
            pixels: paint
                .pix
                .pixels()
                .iter()
                .map(|c| Color32::from_rgba_premultiplied(c.red(), c.green(), c.blue(), c.alpha()))
                .collect(),
        };
        self.tex.set(self.delay_buf.pop_front().unwrap(), TextureOptions::NEAREST);
        self.delay_buf.push_back(img);
        ui.add(
            Image::new(&self.tex)
                .maintain_aspect_ratio(true)
                .shrink_to_fit(),
        );
    }
}
