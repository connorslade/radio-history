use core::f32;
use std::{
    f32::consts::PI,
    fs::File,
    io::BufWriter,
    iter, thread,
    time::{Instant, SystemTime, UNIX_EPOCH},
};

use chrono::{DateTime, NaiveDateTime};
use database::Database;
use hound::{SampleFormat, WavSpec, WavWriter};

use anyhow::Result;
use itertools::Itertools;
use num_complex::Complex;

mod database;
#[cfg(feature = "debug")]
mod debug;
mod filters;
mod transcribe;
mod web;
use filters::{down_sample::DownSampleExt, low_pass::LowPassExt};
use num_traits::Zero;
#[cfg(feature = "debug")]
use rustfft::FftPlanner;
use transcribe::{Transcriber, TRANSCRIBE_SAMPLE_RATE};
use uuid::Uuid;
use web::UiMessage;

const BUFFER_SIZE: usize = 16_384;
const SAMPLE_RATE: u32 = 250_000;
const AUDIO_CUTOFF_FREQ: f32 = 15_000.0;

const SQUELCH: f32 = 0.05;

const WAVE_SAMPLE_RATE: u32 = 44_100;
const WAVE_SPEC: WavSpec = WavSpec {
    channels: 1,
    sample_rate: WAVE_SAMPLE_RATE,
    bits_per_sample: 8,
    sample_format: SampleFormat::Int,
};

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
    thread::spawn(|| debug::start(fft_rx).unwrap());

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
    let mut last_sample = Complex::zero();
    #[cfg(feature = "debug")]
    let mut fft_planner = FftPlanner::new();

    loop {
        let data = device.read_sync(BUFFER_SIZE).unwrap();

        let iq = data
            .chunks_exact(2)
            .map(|chunk| Complex::new(chunk[0] as f32 / 127.5 - 1.0, chunk[1] as f32 / 127.5 - 1.0))
            .collect::<Vec<_>>();

        #[cfg(feature = "debug")]
        {
            let mut fft = iq.clone();
            fft_planner
                .plan_fft_forward(BUFFER_SIZE as usize / 2)
                .process(&mut fft);
            fft_tx.send(fft).unwrap();
        }

        if rms(&iq) < SQUELCH {
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

        let pcm = iter::once(last_sample)
            .chain(iq.into_iter())
            .low_pass(SAMPLE_RATE, 20_000.0)
            .tuple_windows()
            .map(|(a, b)| {
                last_sample = b;

                let mut angle = b.arg() - a.arg();
                if angle > PI {
                    angle -= 2.0 * PI;
                } else if angle < -PI {
                    angle += 2.0 * PI;
                }

                angle * 10.0
            });

        let audio = pcm
            .low_pass(SAMPLE_RATE, AUDIO_CUTOFF_FREQ)
            .down_sample(SAMPLE_RATE, WAVE_SAMPLE_RATE)
            .collect::<Vec<_>>();

        let mean = audio.iter().sum::<f32>() / audio.len() as f32;
        audio.iter().for_each(|x| {
            message
                .wav
                .write_sample(((x - mean) * i8::MAX as f32) as i8)
                .unwrap()
        });

        message.buffer.extend(
            audio
                .into_iter()
                .down_sample(WAVE_SAMPLE_RATE, TRANSCRIBE_SAMPLE_RATE),
        );
    }
}

fn rms(iq: &[Complex<f32>]) -> f32 {
    (iq.iter().map(|c| c.re * c.re + c.im * c.im).sum::<f32>() / iq.len() as f32).sqrt()
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
