use std::{
    fs::File,
    io::{self, BufReader},
    path::Path,
    time::Duration,
};

use egui::{Button, Color32, Slider, TextEdit, Ui};
use hound::WavReader;

use super::TemplateApp;

#[derive(serde::Deserialize, serde::Serialize, Default)]
pub struct AudioInput {
    wav_path: String,
    #[serde(skip)]
    source: Option<AudioSource>,
}

pub struct AudioSource {
    reader: WavReader<BufReader<File>>,
    samples_read: u32,
    samples: u32,
    playing: bool,
}

impl AudioSource {
    pub fn new(reader: WavReader<BufReader<File>>) -> Self {
        Self {
            samples: reader.len(),
            reader,
            samples_read: 0,
            playing: false,
        }
    }

    pub fn read_samples(&mut self, n: u32) -> Vec<i16> {
        let v = self
            .reader
            .samples()
            .take(n as usize)
            .map_while(|r| r.ok())
            .collect::<Vec<_>>();
        self.samples_read += v.len() as u32;
        v
    }
}

impl TemplateApp {
    pub fn ui_audio_in(&mut self, ui: &mut Ui) {
        let audio_in = &mut self.audio_input;
        ui.heading("Audio Input");
        ui.horizontal(|ui| {
            ui.label(".wav path");
            ui.add(TextEdit::singleline(&mut audio_in.wav_path));
        });

        let path = Path::new(&audio_in.wav_path);
        if let Ok(file) = File::open(path)
            && let Ok(reader) = hound::WavReader::new(BufReader::new(file))
            && path.extension().and_then(|s| s.to_str()) == Some("wav")
        {
            if audio_in.source.is_none() {
                audio_in.source = Some(AudioSource::new(reader));
            }
        } else {
            audio_in.source = None;
        }

        if let Some(source) = &mut audio_in.source {
            ui.horizontal(|ui| {
                let text = if source.playing { "⏸" } else { "⏵" };
                let play_toggle = ui.button(text);

                if play_toggle.clicked() {
                    source.playing = !source.playing;
                }

                let mut progress = source.samples_read as f32 / source.samples as f32;
                let slider: egui::Response =
                    ui.add(Slider::new(&mut progress, 0.0..=1.0).show_value(false));

                if slider.changed() {
                    source.playing = false;
                    let t = (progress * source.samples as f32) as u32;
                    source.reader.seek(t).unwrap();
                    source.samples_read = t;
                }

                let time = |s: u32| {
                    let secs = (s as f32 / 44_100.) as u32;
                    format!("{}:{:02}", secs / 60, secs % 60)
                };
                ui.label(format!(
                    "{} / {}",
                    time(source.samples_read),
                    time(source.samples)
                ));
            });
        } else {
            ui.horizontal(|ui| {
                ui.add_enabled(false, Button::new("⏵"));
                ui.add_enabled(false, Slider::new(&mut 0.0, 0.0..=1.0).show_value(false));
                ui.label("?:?? / ?:??");
            });
        }
    }
}
