use std::{
    collections::VecDeque,
    fs::{self, File},
    io::{self, BufReader},
    path::{Path, PathBuf},
    sync::{Arc, OnceLock, mpsc::Sender},
    time::Duration,
};

use egui::{Button, Checkbox, ComboBox, Slider, TextEdit, Ui, mutex::Mutex};
use lib::{
    Complex, Fft, FftPlanner,
    cfg::AnalysisConfig,
    state::{AnalysisState, RawSpec},
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
    pub hps: bool,
    pub harmonic: bool,
    pub percussive: bool,
    pub residual: bool,
    #[serde(skip)]
    pub ifft: Option<Arc<dyn Fft<f32>>>,
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

    let hps = ui.checkbox(&mut audio.hps, "HPS");
    ui.indent("audio_hps", |ui| {
        let h = ui.add_enabled(audio.hps, Checkbox::new(&mut audio.harmonic, "Harmonic"));
        let r = ui.add_enabled(audio.hps, Checkbox::new(&mut audio.residual, "Residual"));
        let p = ui.add_enabled(
            audio.hps,
            Checkbox::new(&mut audio.percussive, "Percussive"),
        );
        if [hps, h, r, p].iter().any(|r| r.clicked()) {
            playback
                .decoder
                .as_mut()
                .map(|d| d.try_seek(Duration::from_secs_f32(audio.progress)).unwrap());
            playback.sink.clear();
        }
    });
}

pub fn playback(
    cfg: &AnalysisConfig,
    audio: &mut Audio,
    playback: &mut Playback,
    state: &AnalysisState,
) {
    // *sigh* okay this is very jank but the vec cannot be sent between threads
    // because the callback in `EmptyCallback` has to satisfy `Fn`
    static SAMPLE_QUEUE: OnceLock<Mutex<VecDeque<Vec<i16>>>> = OnceLock::new();
    static SAMPLE_TX: OnceLock<Sender<Vec<i16>>> = OnceLock::new();

    // we may clear the playback sink in audio::ui
    // not robust to other modifications but its fiiiiine
    if playback.sink.len() == 0
        && let Some(q) = SAMPLE_QUEUE.get()
    {
        q.lock().clear();
    }

    if audio.playing {
        let Some(decoder) = playback.decoder.as_mut() else {
            playback.decoder = None;
            return;
        };

        let hop_len = cfg.fft.hop_len;
        let target_samples = decoder.sample_rate() as usize / hop_len;

        while playback.sink.len() < target_samples * 2 {
            let samples = decoder.take(hop_len).collect::<Vec<_>>();
            let buffer = SamplesBuffer::new(
                decoder.channels(),
                decoder.sample_rate(),
                samples.as_slice(),
            );

            if audio.hps {
                // let mut spec = RawSpec::<Complex<f32>>::blank_default(cfg);
                // for (i, val) in spec.audible_slice_mut(cfg).iter_mut().enumerate() {
                //     *val += state.hps.harmonic[i] * audio.harmonic as u32 as f32;
                //     *val += state.hps.residual[i] * audio.residual as u32 as f32;
                //     *val += state.hps.percussive[i] * audio.percussive as u32 as f32;
                // }

                let ifft = match &audio.ifft {
                    Some(ifft) => ifft.clone(),
                    None => {
                        let ifft = FftPlanner::new().plan_fft_inverse(cfg.fft.frame_len);
                        audio.ifft = Some(ifft.clone());
                        ifft
                    }
                };
                let samples =
                    lib::state::fft::istft_samples(ifft.as_ref(), state.fft.raw.0.clone(), cfg.fft.hop_len)
                        .take(hop_len)
                        .collect::<Vec<_>>();
                let buffer = SamplesBuffer::new(
                    decoder.channels(),
                    decoder.sample_rate(),
                    samples.as_slice(),
                );
                playback.sink.append(buffer);
            } else {
                playback.sink.append(buffer);
            }

            if samples.len() == hop_len {
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
