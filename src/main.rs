use std::{
    fs::File,
    io::BufWriter,
    time::{Instant, SystemTime, UNIX_EPOCH},
};

use anyhow::Result;
use chrono::{DateTime, NaiveDateTime};
use core::f32;
use database::Database;
use demodulate::Demodulator;
use hound::WavWriter;
#[cfg(feature = "debug")]
use rustfft::FftPlanner;
use uuid::Uuid;

mod database;
#[cfg(feature = "debug")]
mod debug;
mod filters;
mod transcribe;
mod web;
use filters::down_sample::DownSampleExt;
use transcribe::{Transcriber, TRANSCRIBE_SAMPLE_RATE};
use web::UiMessage;
mod consts;
mod demodulate;
use consts::{BUFFER_SIZE, SAMPLE_RATE, SQUELCH, WAVE_SAMPLE_RATE, WAVE_SPEC};

struct Message {
    uuid: Uuid,
    wav: WavWriter<BufWriter<File>>,
    buffer: Vec<f32>,
}

fn main() -> Result<()> {
    let mut device = rtlsdr::open(0).unwrap();
    println!("{:?}", device.get_tuner_gains().unwrap());

    #[cfg(feature = "debug")]
    let (fft_tx, fft_rx) = flume::unbounded();
    #[cfg(feature = "debug")]
    std::thread::spawn(|| debug::start(fft_rx).unwrap());

    device.set_center_freq(156_450_000).unwrap();
    device.set_sample_rate(SAMPLE_RATE).unwrap();
    device.set_tuner_gain_mode(true).unwrap();
    device.set_agc_mode(false).unwrap();
    device.set_tuner_gain(10).unwrap();
    device.reset_buffer().unwrap();

    let database = Database::new()?;
    let tx = web::start(database.clone());

    let mut transcriber = Transcriber::new("tiny_en.bin").unwrap();
    let mut wav: Option<Message> = None;
    #[cfg(feature = "debug")]
    let mut fft_planner = FftPlanner::new();

    let mut demod = Demodulator::empty();
    loop {
        let data = device.read_sync(BUFFER_SIZE).unwrap();
        demod.replace(&data);

        #[cfg(feature = "debug")]
        {
            let mut fft = demod.iq().to_owned();
            fft_planner
                .plan_fft_forward(BUFFER_SIZE as usize / 2)
                .process(&mut fft);
            fft_tx.send(fft).unwrap();
        }

        if demod.rms() < SQUELCH {
            if let Some(Message { uuid, wav, buffer }) = wav.take() {
                tx.send(UiMessage::Processing)?;
                wav.finalize().unwrap();

                let start = Instant::now();
                let text = (!buffer.is_empty()).then(|| transcriber.transcribe(&buffer).unwrap());
                let text_ref = text.as_deref();

                println!(
                    "{} ({:?})",
                    text_ref.unwrap_or("<No Text>"),
                    start.elapsed()
                );
                database.lock().insert_message(text_ref, uuid)?;

                tx.send(UiMessage::Complete(database::Message {
                    date: date_time(),
                    audio: uuid,
                    text,
                }))?;
            }
            continue;
        }

        let message = wav.get_or_insert_with(|| {
            tx.send(UiMessage::Receiving).unwrap();

            let uuid = Uuid::new_v4();
            let wav = WavWriter::create(format!("data/audio/{uuid}.wav"), WAVE_SPEC).unwrap();

            Message {
                uuid,
                wav,
                buffer: Vec::new(),
            }
        });

        let audio = demod.to_audio(10.0);

        for sample in &audio {
            message
                .wav
                .write_sample((sample * i8::MAX as f32) as i8)
                .unwrap()
        }

        message.buffer.extend(
            audio
                .into_iter()
                .down_sample(WAVE_SAMPLE_RATE, TRANSCRIBE_SAMPLE_RATE),
        );
    }
}

fn date_time() -> NaiveDateTime {
    DateTime::from_timestamp(
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64,
        0,
    )
    .unwrap()
    .naive_local()
}
