use egui::{Color32, ColorImage, Context, Image, TextureHandle, TextureOptions, Ui};
use lib::state::light::LightConfig;

pub struct Light {
    tex: TextureHandle,
    img: ColorImage,
}

impl Light {
    pub fn new(ctx: &Context, cfg: &LightConfig) -> Self {
        let img = ColorImage::new([cfg.width as usize, cfg.height as usize], Color32::BLACK);
        Self {
            tex: ctx.load_texture("light", img.clone(), TextureOptions::NEAREST),
            img,
        }
    }

    pub fn ui(&mut self, _ctx: &Context, ui: &mut Ui, _cfg: &LightConfig) {
        self.tex.set(self.img.clone(), TextureOptions::NEAREST);
        ui.add(
            Image::new(&self.tex)
                .maintain_aspect_ratio(true)
                .shrink_to_fit(),
        );
    }
}
