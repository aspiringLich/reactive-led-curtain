use std::{
    collections::VecDeque,
    fs::{self, File},
    io::{self, BufReader},
    path::{Path, PathBuf},
    sync::{
        OnceLock,
        mpsc::{Receiver, Sender},
    },
    time::Duration,
};

use egui::{Button, Checkbox, ComboBox, Slider, TextEdit, Ui, mutex::Mutex};
use lib::{
    Complex,
    cfg::AnalysisConfig,
    state::{AnalysisState, RawSpec, fft},
};
use rodio::{
    Decoder, OutputStream, Sink, Source,
    buffer::SamplesBuffer,
    source::{EmptyCallback, TrackPosition},
};

type AudioDecoder = TrackPosition<Decoder<BufReader<File>>>;

#[derive(serde::Deserialize, serde::Serialize, Default)]
#[serde(default)]
pub struct Audio {
    folder: String,
    file: String,
    pub playing: bool,
    progress: f32,
    loop_audio: bool,
    pub hps: bool,
    pub harmonic: bool,
    pub percussive: bool,
    pub residual: bool,
}

impl Audio {
    pub fn filepath(&self) -> PathBuf {
        return PathBuf::from(format!("{}/{}", self.folder, self.file));
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
    ui.style_mut().spacing.combo_height = f32::INFINITY;

    ui.horizontal(|ui| {
        ui.label("Folder");
        let folder_edit = ui.add(TextEdit::singleline(&mut audio.folder));

        if folder_edit.changed() {
            audio.file = String::new();
        }
    });
    let file_select = ComboBox::from_label("File")
        .selected_text(&audio.file)
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
                playback.dummy_sink.clear();
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

    ui.checkbox(&mut audio.loop_audio, "Loop");

    let hps = ui.checkbox(&mut audio.hps, "HPS");
    ui.indent("audio_hps", |ui| {
        let h = ui.add_enabled(audio.hps, Checkbox::new(&mut audio.harmonic, "Harmonic"));
        let r = ui.add_enabled(audio.hps, Checkbox::new(&mut audio.residual, "Residual"));
        let p = ui.add_enabled(
            audio.hps,
            Checkbox::new(&mut audio.percussive, "Percussive"),
        );
        if [hps, h, r, p].iter().any(|r| r.changed()) {
            playback
                .decoder
                .as_mut()
                .map(|d| d.try_seek(Duration::from_secs_f32(audio.progress)).unwrap());
            playback.dummy_sink.clear();
        }
    });
}

pub struct Playback {
    pub decoder: Option<AudioDecoder>,
    pub dummy_sink: Sink,
    audio_sink: Sink,
    _stream: OutputStream, // DONT DROP
    sample_tx: Sender<Vec<i16>>,
    audio_rx: Receiver<Vec<i16>>,
    istft: fft::InverseStft,
}

impl Playback {
    pub fn new(
        audio: &mut Audio,
        sample_tx: Sender<Vec<i16>>,
        audio_rx: Receiver<Vec<i16>>,
        cfg: &AnalysisConfig,
    ) -> Self {
        let stream = rodio::OutputStreamBuilder::open_default_stream().unwrap();
        let decoder = try_get_decoder(&audio.filepath(), audio.progress);
        if decoder.is_none() {
            audio.playing = false;
        }

        let dummy_sink = Sink::connect_new(&stream.mixer());
        dummy_sink.set_volume(0.0);
        let audio_sink = Sink::connect_new(&stream.mixer());

        let istft = fft::InverseStft::new(&cfg);

        Self {
            decoder,
            dummy_sink,
            audio_sink,
            _stream: stream,
            sample_tx,
            audio_rx,
            istft,
        }
    }

    /// Call this when samples are received by `spectrogram` to modify the
    /// samples to be played if necessary
    pub fn audio_samples(
        &mut self,
        audio: &Audio,
        samples: Vec<i16>,
        cfg: &AnalysisConfig,
        state: &AnalysisState,
    ) -> Vec<i16> {
        if audio.hps {
            let mut spec = RawSpec::<Complex<f32>>::blank_default(cfg);
            for (i, val) in spec.audible_slice_mut(cfg).iter_mut().enumerate() {
                *val += state.hps.harmonic[i] * audio.harmonic as u32 as f32;
                *val += state.hps.residual[i] * audio.residual as u32 as f32;
                *val += state.hps.percussive[i] * audio.percussive as u32 as f32;
            }

            self.istft.push(spec.0).collect()
        } else {
            samples
        }
    }
}

pub fn playback(cfg: &AnalysisConfig, audio: &mut Audio, playback: &mut Playback) {
    // *sigh* okay this is very jank but the vec cannot be sent between threads
    // because the callback in `EmptyCallback` has to satisfy `Fn`
    static SAMPLE_QUEUE: OnceLock<Mutex<VecDeque<Vec<i16>>>> = OnceLock::new();
    static SAMPLE_TX: OnceLock<Sender<Vec<i16>>> = OnceLock::new();

    // we may clear the playback sink in audio::ui
    // not robust to other modifications but its fiiiiine
    if playback.dummy_sink.len() == 0
        && let Some(q) = SAMPLE_QUEUE.get()
    {
        q.lock().clear();
    }

    let Some(decoder) = playback.decoder.as_mut() else {
        playback.decoder = None;
        return;
    };

    if audio.playing {
        let hop_len = cfg.fft.hop_len;
        let target_samples = decoder.sample_rate() as usize / hop_len;

        while playback.dummy_sink.len() < target_samples * 2 {
            let samples = decoder.take(hop_len).collect::<Vec<_>>();

            // if there are no more samples left to read
            if samples.len() == 0 {
                if audio.loop_audio {
                    decoder.try_seek(Duration::from_secs_f32(0.0)).unwrap();
                    continue;
                } else {
                    audio.playing = false;
                    break;
                }
            }

            let buffer = SamplesBuffer::new(
                decoder.channels(),
                decoder.sample_rate(),
                vec![0.0; samples.len()],
            );
            playback.dummy_sink.append(buffer);

            if samples.len() == hop_len {
                SAMPLE_QUEUE
                    .get_or_init(|| Default::default())
                    .lock()
                    .push_back(samples);
                SAMPLE_TX.get_or_init(|| playback.sample_tx.clone());

                playback
                    .dummy_sink
                    .append(EmptyCallback::<f32>::new(Box::new(|| {
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
        playback.dummy_sink.play();
    } else {
        playback.dummy_sink.pause();
    }

    // actually play audio here (xd)
    while let Ok(a) = playback.audio_rx.try_recv() {
        let buffer = SamplesBuffer::new(decoder.channels(), decoder.sample_rate(), a.as_slice());
        playback.audio_sink.append(buffer);
    }
}
