use std::sync::mpsc::Receiver;

use egui::{Color32, ColorImage, Context, TextureHandle, TextureOptions, Ui};

use crate::SAMPLE_SIZE;

pub struct Spectrogram {
    tex: TextureHandle,
    sample_rx: Receiver<Vec<i16>>,
}

impl Spectrogram {
    pub fn new(ctx: &Context, sample_rx: Receiver<Vec<i16>>) -> Self {
        let img = ColorImage::new([SAMPLE_SIZE, SAMPLE_SIZE], Color32::BLACK);
        Self {
            tex: ctx.load_texture("spectrogram", img, TextureOptions::LINEAR),
            sample_rx,
        }
    }

    pub fn ui(&self, ui: &mut Ui) {
        while let Ok(sample) = self.sample_rx.try_recv() {
            dbg!(sample[0]);
        }
        ui.image(&self.tex);
    }
}
