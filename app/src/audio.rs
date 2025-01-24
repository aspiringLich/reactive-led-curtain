use std::{
    collections::VecDeque,
    fs::File,
    io::BufReader,
    path::Path,
    sync::{OnceLock, mpsc::Sender},
    time::Duration,
};

use egui::{Button, Slider, TextEdit, Ui, mutex::Mutex};
use rodio::{
    Decoder, OutputStream, Sink, Source,
    buffer::SamplesBuffer,
    source::{EmptyCallback, TrackPosition},
};

use lib::SAMPLE_SIZE;

type AudioDecoder = TrackPosition<Decoder<BufReader<File>>>;

#[derive(serde::Deserialize, serde::Serialize, Default)]
pub struct Audio {
    wav_path: String,
    pub playing: bool,
    progress: f32,
}

pub struct Playback {
    decoder: Option<AudioDecoder>,
    sink: Sink,
    _stream: OutputStream, // DONT DROP
    sample_tx: Sender<Vec<i16>>,
}

impl Playback {
    pub fn new(audio: &Audio, sample_tx: Sender<Vec<i16>>) -> Self {
        let stream = rodio::OutputStreamBuilder::open_default_stream().unwrap();
        Self {
            decoder: try_get_decoder(Path::new(&audio.wav_path), audio.progress),
            sink: Sink::connect_new(&stream.mixer()),
            _stream: stream,
            sample_tx,
        }
    }
}

fn try_get_decoder(path: &Path, progress: f32) -> Option<AudioDecoder> {
    let file = File::open(path).ok()?;
    let decoder = Decoder::new(BufReader::new(file)).ok()?;
    if path.extension().and_then(|s| s.to_str()) == Some("wav") {
        let mut decoder = decoder.track_position();
        decoder.try_seek(Duration::from_secs_f32(progress)).ok()?;
        Some(decoder)
    } else {
        None
    }
}

pub fn ui(ui: &mut Ui, audio: &mut Audio, playback: &mut Playback) {
    ui.heading("Playback");
    ui.horizontal(|ui| {
        ui.label("filepath");
        let wav_path_edit = ui.add(TextEdit::singleline(&mut audio.wav_path));

        let path = Path::new(&audio.wav_path);
        if wav_path_edit.changed() {
            audio.progress = 0.0;
            playback.decoder = try_get_decoder(path, audio.progress);
        }
    });

    if let Some(decoder) = &mut playback.decoder {
        ui.horizontal(|ui| {
            let text = if audio.playing { "⏸" } else { "⏵" };
            let play_toggle = ui.button(text);

            let mut total_duration = decoder.total_duration().unwrap().as_secs_f32();

            if play_toggle.clicked() {
                audio.playing = !audio.playing;
            }

            audio.progress = decoder.get_pos().as_secs_f32();
            if total_duration < audio.progress {
                total_duration = audio.progress;
            }
            let slider: egui::Response =
                ui.add(Slider::new(&mut audio.progress, 0.0..=total_duration).show_value(false));

            if slider.changed() {
                decoder
                    .try_seek(Duration::from_secs_f32(audio.progress))
                    .unwrap();
                playback.sink.clear();
                if let Some(mut queue) = SAMPLE_QUEUE.get().map(|q| q.lock()) {
                    queue.clear();
                }
            }

            let time = |s: f32| format!("{}:{:02}", s as u32 / 60, s as u32 % 60);
            ui.monospace(format!(
                "{} / {}",
                time(audio.progress),
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

    // *sigh* okay this is very jank but the vec cannot be sent between threads
    // because the callback in `EmptyCallback` has to satisfy `Fn`
    static SAMPLE_QUEUE: OnceLock<Mutex<VecDeque<Vec<i16>>>> = OnceLock::new();
    static SAMPLE_TX: OnceLock<Sender<Vec<i16>>> = OnceLock::new();

    if audio.playing {
        let decoder = playback.decoder.as_mut().unwrap();
        let target_samples = decoder.sample_rate() as usize / SAMPLE_SIZE;
        while playback.sink.len() < target_samples {
            let samples = decoder.take(SAMPLE_SIZE).collect::<Vec<_>>();
            let buffer = SamplesBuffer::new(
                decoder.channels(),
                decoder.sample_rate(),
                samples.as_slice(),
            );

            playback.sink.append(buffer);
            if samples.len() == SAMPLE_SIZE {
                SAMPLE_QUEUE
                    .get_or_init(|| Default::default())
                    .lock()
                    .push_back(samples);
                SAMPLE_TX.get_or_init(|| playback.sample_tx.clone());

                playback.sink.append(EmptyCallback::<f32>::new(Box::new(|| {
                    // indubitably the best code to ever grace god's green earth
                    SAMPLE_TX
                        .get()
                        .expect("SAMPLE_TX should be initialized")
                        .send(
                            SAMPLE_QUEUE
                                .get()
                                .expect("SAMPLE_QUEUE should be initialized")
                                .lock()
                                .pop_front()
                                .expect("SAMPLE_QUEUE should never be empty"),
                        )
                        .unwrap();
                })));
            }
        }
        playback.sink.play();
    } else {
        playback.sink.pause();
    }
}
