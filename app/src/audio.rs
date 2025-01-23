use std::{fs::File, io::BufReader, path::Path, time::Duration};

use egui::{Button, Slider, TextEdit, Ui};
use rodio::{Decoder, OutputStream, Sink, Source, buffer::SamplesBuffer, source::TrackPosition};

type AudioDecoder = TrackPosition<Decoder<BufReader<File>>>;

#[derive(serde::Deserialize, serde::Serialize, Default)]
pub struct Audio {
    wav_path: String,
    pub playing: bool,
    progress: f32,

    #[serde(skip)]
    decoder: Option<AudioDecoder>,
    #[serde(skip)]
    sink: Option<(Sink, OutputStream)>,
}

impl Audio {
    pub fn ui(&mut self, ui: &mut Ui) {
        ui.heading("Audio");
        ui.horizontal(|ui| {
            ui.label("filepath");
            ui.add(TextEdit::singleline(&mut self.wav_path));
        });

        let path = Path::new(&self.wav_path);
        if let Ok(file) = File::open(path)
            && let Ok(decoder) = Decoder::new(BufReader::new(file))
            && path.extension().and_then(|s| s.to_str()) == Some("wav")
        {
            if self.decoder.is_none() {
                self.decoder = Some(decoder.track_position());
                if self.progress != Default::default() {
                    self.decoder
                        .as_mut()
                        .map(|d| d.try_seek(Duration::from_secs_f32(self.progress)));
                }
            }
        } else {
            self.decoder = None;
        }

        if let Some(decoder) = &mut self.decoder {
            ui.horizontal(|ui| {
                let text = if self.playing { "⏸" } else { "⏵" };
                let play_toggle = ui.button(text);

                let mut total_duration = decoder.total_duration().unwrap().as_secs_f32();

                if play_toggle.clicked() {
                    self.playing = !self.playing;
                }

                self.progress = decoder.get_pos().as_secs_f32();
                if total_duration < self.progress {
                    total_duration = self.progress;
                }
                let slider: egui::Response =
                    ui.add(Slider::new(&mut self.progress, 0.0..=total_duration).show_value(false));

                if slider.changed() {
                    decoder
                        .try_seek(Duration::from_secs_f32(self.progress))
                        .unwrap();
                    if let Some(s) = self.sink.as_mut() {
                        s.0.clear()
                    }
                }

                let time = |s: f32| format!("{}:{:02}", s as u32 / 60, s as u32 % 60);
                ui.monospace(format!(
                    "{} / {}",
                    time(self.progress),
                    time(total_duration)
                ));
            });
        } else {
            ui.horizontal(|ui| {
                ui.add_enabled(false, Button::new("⏵"));
                ui.add_enabled(false, Slider::new(&mut 0.0, 0.0..=1.0).show_value(false));
                ui.monospace("?:?? / ?:??");
            });
        }

        if self.playing
            && let Some(decoder) = &mut self.decoder
        {
            if self.sink.is_none() {
                let stream = rodio::OutputStreamBuilder::open_default_stream().unwrap();
                self.sink = Some((Sink::connect_new(&stream.mixer()), stream));
            }
            if let Some((sink, _)) = &self.sink {
                let sample_size = 2048;
                let target_samples = decoder.sample_rate() as usize / sample_size;
                while sink.len() < target_samples {
                    let samples = decoder
                        .take(sample_size)
                        .collect::<Vec<_>>();
                    let buffer =
                        SamplesBuffer::new(decoder.channels(), decoder.sample_rate(), samples);

                    sink.append(buffer);
                    sink.play();
                }
            }
        } else if let Some((sink, _)) = &self.sink {
            sink.pause();
        }
    }
}
