use std::{
    collections::VecDeque,
    fs::{self, File},
    io::{self, BufReader},
    path::{Path, PathBuf},
    sync::{OnceLock, mpsc::Sender},
    time::Duration,
};

use egui::{Button, ComboBox, Slider, TextEdit, Ui, mutex::Mutex};
use rodio::{
    Decoder, OutputStream, Sink, Source,
    buffer::SamplesBuffer,
    source::{EmptyCallback, TrackPosition},
};

use lib::{SAMPLE_SIZE, SAMPLE_WINDOWS};

type AudioDecoder = TrackPosition<Decoder<BufReader<File>>>;

#[derive(serde::Deserialize, serde::Serialize, Default)]
#[serde(default)]
pub struct Audio {
    folder: String,
    file: String,
    pub playing: bool,
    progress: f32,
}

impl Audio {
    pub fn filepath(&self) -> PathBuf {
        return PathBuf::from(format!("{}/{}", self.folder, self.file));
    }
}

pub struct Playback {
    decoder: Option<AudioDecoder>,
    sink: Sink,
    _stream: OutputStream, // DONT DROP
    sample_tx: Sender<Vec<i16>>,
}

impl Playback {
    pub fn new(audio: &mut Audio, sample_tx: Sender<Vec<i16>>) -> Self {
        let stream = rodio::OutputStreamBuilder::open_default_stream().unwrap();
        let decoder = try_get_decoder(&audio.filepath(), audio.progress);
        if decoder.is_none() {
            audio.playing = false;
        }
        Self {
            decoder,
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

fn read_dir(dir: &Path) -> io::Result<Vec<String>> {
    let mut out = fs::read_dir(dir)?
        .filter_map(|entry| Some(entry.ok()?))
        .filter(|entry| entry.path().is_file())
        .filter_map(|entry| entry.file_name().into_string().ok())
        .collect::<Vec<_>>();
    out.sort_by_key(|s| s.to_lowercase());
    Ok(out)
}

pub fn ui(ui: &mut Ui, audio: &mut Audio, playback: &mut Playback) {
    ui.heading("Playback");
    ui.horizontal(|ui| {
        ui.label("Folder");
        let folder_edit = ui.add(TextEdit::singleline(&mut audio.folder));

        if folder_edit.changed() {
            audio.file = String::new();
        }
    });
    let file_select = ComboBox::from_label("File")
        .selected_text(&audio.file)
        .height(f32::INFINITY)
        .show_ui(ui, |ui| {
            if let Ok(files) = read_dir(&Path::new(&audio.folder)) {
                files.into_iter().any(|file| {
                    ui.selectable_value(&mut audio.file, file.clone(), &file)
                        .clicked()
                })
            } else {
                false
            }
        });
    if file_select.inner == Some(true) {
        audio.progress = 0.0;
        playback.decoder = try_get_decoder(&audio.filepath(), audio.progress);
    }

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
        let Some(decoder) = playback.decoder.as_mut() else {
            playback.decoder = None;
            return;
        };

        let target_sample_size = SAMPLE_SIZE / SAMPLE_WINDOWS;
        let target_samples = decoder.sample_rate() as usize / target_sample_size;

        while playback.sink.len() < target_samples * 2 {
            let samples = decoder.take(target_sample_size).collect::<Vec<_>>();
            let buffer = SamplesBuffer::new(
                decoder.channels(),
                decoder.sample_rate(),
                samples.as_slice(),
            );

            playback.sink.append(buffer);
            if samples.len() == target_sample_size {
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
