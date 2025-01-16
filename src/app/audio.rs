use std::{fs::File, io::BufReader, path::Path, thread::{self}, time::Duration};

use egui::{Button, Slider, TextEdit, Ui};
use rodio::{Decoder, OutputStream, Sink, Source, buffer::SamplesBuffer, source::TrackPosition};

type AudioDecoder = TrackPosition<Decoder<BufReader<File>>>;

#[derive(serde::Deserialize, serde::Serialize, Default)]
pub struct Audio {
    wav_path: String,
    playing: bool,

    #[serde(skip)]
    decoder: Option<AudioDecoder>,
    #[serde(skip)]
    sink: Option<(Sink, OutputStream)>,
}

// #[derive(serde::Serialize, serde::Deserialize, Default)]
// #[serde(default)]
// pub struct AudioSource {
//
//     samples_read: u32,
//     samples: u32,
//     playing: bool,
// }

// impl AudioSource {
//     pub fn new(decoder: Decoder<BufReader<File>>) -> Self {
//         Self {
//             samples: decoder,
//             decoder,
//             samples_read: 0,
//             playing: false,
//         }
//     }

//     pub fn read_samples(&mut self, n: u32) -> Vec<i16> {
//         let v = self
//             .decoder
//             .samples()
//             .take(n as usize)
//             .map_while(|r| r.ok())
//             .collect::<Vec<_>>();
//         self.samples_read += v.len() as u32;
//         v
//     }
// }

impl Audio {
    pub fn ui(&mut self, ui: &mut Ui) {
        ui.heading("Audio");
        ui.horizontal(|ui| {
            ui.label(".wav path");
            ui.add(TextEdit::singleline(&mut self.wav_path));
        });

        let path = Path::new(&self.wav_path);
        if let Ok(file) = File::open(path)
            && let Ok(decoder) = Decoder::new(BufReader::new(file))
            && path.extension().and_then(|s| s.to_str()) == Some("wav")
        {
            if self.decoder.is_none() {
                self.decoder = Some(decoder.track_position());
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

                let mut progress = decoder.get_pos().as_secs_f32();
                if total_duration < progress {
                    total_duration = progress;
                }
                let slider: egui::Response =
                    ui.add(Slider::new(&mut progress, 0.0..=total_duration).show_value(false));

                if slider.changed() {
                    self.playing = false;
                    decoder.try_seek(Duration::from_secs_f32(progress)).unwrap();
                }

                let time = |s: f32| format!("{}:{:02}", s as u32 / 60, s as u32 % 60);
                ui.monospace(format!("{} / {}", time(progress), time(total_duration)));
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
                if sink.len() <= 1 {
                    let samples = decoder
                        .take_duration(Duration::from_secs_f32(1.0 / 6.0))
                        .collect::<Vec<_>>();
                    let buffer = SamplesBuffer::new(
                        decoder.channels(),
                        decoder.sample_rate(),
                        samples
                    );
                    
                    sink.append(buffer);
                    sink.play();
                }
            }
        } else if let Some((sink, _)) = &self.sink {
            sink.pause();
        }
    }
}
