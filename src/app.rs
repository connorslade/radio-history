use std::{fs::File, io::BufWriter, path::Path, time::Instant};

use anyhow::Result;
use hound::WavWriter;
use rtlsdr::RTLSDRDevice;
use uuid::Uuid;

use crate::{
    config::Config,
    consts::{BUFFER_SIZE, WAVE_SAMPLE_RATE, WAVE_SPEC},
    filters::down_sample::DownSampleExt,
    misc::date_time,
    signal::{
        demodulate::Demodulator,
        transcribe::{Transcriber, TRANSCRIBE_SAMPLE_RATE},
    },
    web::{
        self,
        database::{self, Database},
        UiMessage,
    },
};

pub struct App {
    config: Config,

    device: RTLSDRDevice,
    demodulator: Demodulator,
    recordings: Vec<Option<Message>>,
    #[cfg(feature = "debug")]
    debug: flume::Sender<Vec<num_complex::Complex<f32>>>,

    database: Database,
    transcriber: Transcriber,
    web_tx: flume::Sender<UiMessage>,
}

struct Message {
    uuid: Uuid,
    wav: WavWriter<BufWriter<File>>,
    buffer: Vec<f32>,
}

impl App {
    pub fn new(config: Config) -> Result<Self> {
        let device = rtlsdr::open(config.radio.device_index).unwrap();
        let demodulator = Demodulator::empty();
        let recordings = (0..config.channels.len()).map(|_| None).collect::<Vec<_>>();

        let database = Database::new(&config.misc.data_dir)?;
        let transcriber = Transcriber::new(&config.misc.transcribe_model)?;
        let web_tx = web::start(&config.server, database.clone());

        #[cfg(feature = "debug")]
        let debug_tx = {
            let (tx, rx) = flume::unbounded();
            std::thread::spawn(move || crate::signal::debug::start(rx).unwrap());
            tx
        };

        Ok(Self {
            config,
            device,
            demodulator,
            recordings,
            #[cfg(feature = "debug")]
            debug: debug_tx,
            database,
            transcriber,
            web_tx,
        })
    }

    pub fn init_device(&mut self) {
        let App { config, device, .. } = self;

        device.set_center_freq(config.radio.center_freq).unwrap();
        device.set_sample_rate(config.radio.sample_rate).unwrap();
        device.set_tuner_gain_mode(true).unwrap();
        device.set_agc_mode(false).unwrap();
        device.set_tuner_gain(config.radio.tuner_gain).unwrap();
        device.reset_buffer().unwrap();
    }

    pub fn process_samples(&mut self) {
        let data = self.device.read_sync(BUFFER_SIZE).unwrap();
        self.demodulator.replace(&data);
        #[cfg(feature = "debug")]
        self.debug.send(self.demodulator.iq().to_owned()).unwrap();

        let mut finalize = Vec::new();
        for (idx, channel) in self.config.channels.iter().enumerate() {
            let offset = channel.freq - self.config.radio.center_freq;

            let rms = self.demodulator.rms(offset);
            if rms < channel.squelch {
                finalize.push(idx);
                continue;
            }

            let message = self.recordings[idx].get_or_insert_with(|| {
                self.web_tx
                    .send(UiMessage::Receiving {
                        idx: idx as u32,
                        name: channel.name.to_owned(),
                    })
                    .unwrap();
                Message::new(&self.config.misc.data_dir)
            });

            let audio = self.demodulator.audio(offset, channel.gain);
            message.write_audio(audio);
        }

        for index in finalize {
            self.finalize_recording(index).unwrap();
        }
    }

    fn finalize_recording(&mut self, index: usize) -> Result<()> {
        if let Some(Message { uuid, wav, buffer }) = self.recordings[index].take() {
            self.web_tx
                .send(UiMessage::Processing { idx: index as u32 })?;
            wav.finalize().unwrap();

            let start = Instant::now();
            let text = (!buffer.is_empty()).then(|| self.transcriber.transcribe(&buffer).unwrap());
            let text_ref = text.as_deref();

            println!(
                "{} ({:?})",
                text_ref.unwrap_or("<No Text>"),
                start.elapsed()
            );
            self.database.lock().insert_message(text_ref, uuid)?;

            self.web_tx.send(UiMessage::Complete(database::Message {
                date: date_time(),
                audio: uuid,
                text,
            }))?;
        }

        Ok(())
    }
}

impl Message {
    fn new(data_dir: &Path) -> Self {
        let uuid = Uuid::new_v4();
        let path = data_dir.join("audio").join(format!("{}.wav", uuid));
        let wav = WavWriter::create(path, WAVE_SPEC).unwrap();

        Message {
            uuid,
            wav,
            buffer: Vec::new(),
        }
    }

    fn write_audio(&mut self, audio: Vec<f32>) {
        for sample in &audio {
            self.wav
                .write_sample((sample * i8::MAX as f32) as i8)
                .unwrap()
        }

        self.buffer.extend(
            audio
                .into_iter()
                .down_sample(WAVE_SAMPLE_RATE, TRANSCRIBE_SAMPLE_RATE),
        );
    }
}
