use std::{fs::File, io::BufWriter, time::Instant};

use chrono::Local;
use database::Database;
use hound::{SampleFormat, WavSpec, WavWriter};

use anyhow::Result;
use itertools::Itertools;
use num_complex::{Complex, ComplexFloat};

mod database;
mod filters;
mod transcribe;
mod web;
use filters::{down_sample::DownSampleExt, low_pass::LowPassExt};
use transcribe::{Transcriber, TRANSCRIBE_SAMPLE_RATE};
use uuid::Uuid;

const BUFFER_SIZE: usize = 16_384;
const SAMPLE_RATE: u32 = 250_000;
const AUDIO_CUTOFF_FREQ: f32 = 15_000.0;

const SQUELCH: f32 = 0.7;

const WAVE_SAMPLE_RATE: u32 = 44_100;
const WAVE_SPEC: WavSpec = WavSpec {
    channels: 1,
    sample_rate: WAVE_SAMPLE_RATE,
    bits_per_sample: 32,
    sample_format: SampleFormat::Float,
};

struct Message {
    uuid: Uuid,
    wav: WavWriter<BufWriter<File>>,
    buffer: Vec<f32>,
}

fn main() -> Result<()> {
    let mut device = rtlsdr::open(0).unwrap();

    device.set_center_freq(156_450_000).unwrap();
    device.set_sample_rate(SAMPLE_RATE).unwrap();
    device.reset_buffer().unwrap();

    let database = Database::new()?;
    web::start(database.clone());

    let mut transcriber = Transcriber::new("tiny_en.bin").unwrap();
    let mut wav: Option<Message> = None;

    loop {
        let data = device.read_sync(BUFFER_SIZE).unwrap();
        let iq = data
            .chunks_exact(2)
            .map(|chunk| Complex::new(chunk[0] as f32 / 127.5 - 1.0, chunk[1] as f32 / 127.5 - 1.0))
            .collect::<Vec<_>>();

        if rms(&iq) < SQUELCH {
            if let Some(Message { uuid, wav, buffer }) = wav.take() {
                wav.finalize().unwrap();

                let start = Instant::now();
                let text = (!buffer.is_empty()).then(|| transcriber.transcribe(&buffer).unwrap());
                let text = text.as_deref();

                println!("{} ({:?})", text.unwrap_or("<No Text>"), start.elapsed());
                database.lock().insert_message(text, uuid)?;
            }
            continue;
        }

        let message = wav.get_or_insert_with(|| {
            let uuid = Uuid::new_v4();
            let wav = WavWriter::create(format!("data/audio/{uuid}.wav"), WAVE_SPEC).unwrap();

            Message {
                uuid,
                wav,
                buffer: Vec::new(),
            }
        });

        let pcm = iq
            .into_iter()
            .low_pass(SAMPLE_RATE, 20_000.0)
            .tuple_windows()
            .map(|(i, q)| {
                let c = i * q.conj();
                let angle = f32::atan2(c.im as f32, c.re as f32);
                angle
            });

        let audio = pcm
            .low_pass(SAMPLE_RATE, AUDIO_CUTOFF_FREQ)
            .filter(|x| x.abs() < 1.0)
            .down_sample(SAMPLE_RATE, WAVE_SAMPLE_RATE)
            .collect::<Vec<_>>();

        audio
            .iter()
            .for_each(|x| message.wav.write_sample(*x).unwrap());

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
