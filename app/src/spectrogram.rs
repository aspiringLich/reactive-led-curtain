use std::sync::{mpsc::Receiver};

use egui::{Color32, ColorImage, Context, Image, TextureHandle, TextureOptions, Ui, Vec2};

use lib::{state::AnalysisState, SAMPLE_SIZE};

pub struct Spectrogram {
    tex: TextureHandle,
    img: ColorImage,
    sample_rx: Receiver<Vec<i16>>,
    state: AnalysisState,
}

impl Spectrogram {
    pub fn new(ctx: &Context, sample_rx: Receiver<Vec<i16>>) -> Self {
        let img = ColorImage::new([SAMPLE_SIZE, SAMPLE_SIZE], Color32::BLACK);
        Self {
            tex: ctx.load_texture("spectrogram", img.clone(), TextureOptions::LINEAR),
            img,
            sample_rx,
            state: Default::default()
        }
    }

    pub fn ui(&mut self, ui: &mut Ui) {
        let mut any = false;
        while let Ok(samples) = self.sample_rx.try_recv() {
            self.state = AnalysisState::from_prev(&self.state, &samples);
            let fft_out = &self.state.fft_out;
            dbg!(fft_out.db.iter().take(10).collect::<Vec<_>>());

            // rotating left shifts stuff left; we are overwriting the last column anyway
            //
            // 0 1 2     1 2 |3
            // 3 4 5  -> 4 5 |6
            // 6 7 8     7 8 |0
            self.img.pixels.rotate_left(1);

            // now write fft data in the last column
            for (i, p_idx) in (0..SAMPLE_SIZE)
                .into_iter()
                .map(|i| i * SAMPLE_SIZE + SAMPLE_SIZE - 1)
                .enumerate()
            {
                self.img.pixels[p_idx] =
                    Color32::from_gray((*fft_out.db[i]) as u8);
            }
            any = true;
        }
        if any {
            self.tex.set(self.img.clone(), TextureOptions::LINEAR);
        }
        ui.add(
            Image::new(&self.tex)
                .maintain_aspect_ratio(false)
                .shrink_to_fit(),
        );
    }
}
