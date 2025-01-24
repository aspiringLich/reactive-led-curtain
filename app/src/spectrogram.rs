use std::sync::mpsc::Receiver;

use egui::{Color32, ColorImage, Context, Image, TextureHandle, TextureOptions, Ui, Vec2};

use lib::{
    SAMPLE_SIZE,
    fft::{self, FftOutput},
    state::AnalysisState,
};

pub struct Spectrogram {
    tex: TextureHandle,
    img: ColorImage,
    sample_rx: Receiver<Vec<i16>>,
    state: AnalysisState,
    samples: Vec<i16>,
}

const IMG_WIDTH: usize = 512;
const DFT_IDX: usize = fft::hz_to_idx(10_000.0);

const HOP_LENGTH: usize = 4;

impl Spectrogram {
    pub fn new(ctx: &Context, sample_rx: Receiver<Vec<i16>>) -> Self {
        let img = ColorImage::new([IMG_WIDTH, DFT_IDX], Color32::BLACK);
        Self {
            tex: ctx.load_texture("spectrogram", img.clone(), TextureOptions::LINEAR),
            img,
            sample_rx,
            state: Default::default(),
            samples: vec![0; SAMPLE_SIZE],
        }
    }

    pub fn ui(&mut self, ui: &mut Ui) {
        let mut any = false;
        while let Ok(samples) = self.sample_rx.try_recv() {
            self.state = AnalysisState::from_prev(&self.state, &samples);
            let combined = self
                .samples
                .iter()
                .cloned()
                .chain(samples.iter().cloned())
                .collect::<Vec<_>>();
            assert_eq!(combined.len(), SAMPLE_SIZE * 2);

            for i in 0..HOP_LENGTH {
                let n0 = SAMPLE_SIZE / HOP_LENGTH * i;
                let fft_out = FftOutput::new(
                    self.state.fft_out.fft.clone(),
                    &combined[n0..n0 + SAMPLE_SIZE],
                );

                // rotating left shifts stuff left; we are overwriting the last column anyway
                //
                // 0 1 2     1 2 |3
                // 3 4 5  -> 4 5 |6
                // 6 7 8     7 8 |0
                self.img.pixels.rotate_left(1);

                // now write fft data in the last column
                let [w, h] = self.img.size;
                for (i, p_idx) in (0..DFT_IDX)
                    .into_iter()
                    .map(|i| (h - i) * w - 1)
                    .enumerate()
                {
                    self.img.pixels[p_idx] = Color32::from_gray((*fft_out.db[i]) as u8);
                }
            }
            self.samples = samples;

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
