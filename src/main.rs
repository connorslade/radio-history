use std::{fs::File, io::BufWriter};

use chrono::Local;
use hound::{SampleFormat, WavSpec, WavWriter};

use itertools::Itertools;
use num_complex::{Complex, ComplexFloat};

mod low_pass;
use low_pass::LowPassExt;

const BUFFER_SIZE: usize = 16_384;
const SAMPLE_RATE: u32 = 250_000;
const AUDIO_CUTOFF_FREQ: f32 = 15_000.0;

const SQUELCH: f32 = 0.6;

const WAVE_SAMPLE_RATE: u32 = 44_100;
const WAVE_SPEC: WavSpec = WavSpec {
    channels: 1,
    sample_rate: WAVE_SAMPLE_RATE,
    bits_per_sample: 32,
    sample_format: SampleFormat::Float,
};

fn main() {
    let mut device = rtlsdr::open(0).unwrap();

    device.set_center_freq(156_450_000).unwrap();
    device.set_sample_rate(SAMPLE_RATE).unwrap();
    device.reset_buffer().unwrap();

    let mut wav: Option<WavWriter<BufWriter<File>>> = None;

    loop {
        let data = device.read_sync(BUFFER_SIZE).unwrap();
        let iq = data
            .chunks_exact(2)
            .map(|chunk| Complex::new(chunk[0] as f32 / 128.0 - 1.0, chunk[1] as f32 / 128.0 - 1.0))
            .collect::<Vec<_>>();

        if rms(&iq) < SQUELCH {
            if let Some(wav) = wav.take() {
                wav.finalize().unwrap();
            }
            continue;
        }

        let wav = wav.get_or_insert_with(|| {
            let filename = Local::now().format("rec/%Y-%m-%d_%H-%M-%S.wav").to_string();
            WavWriter::create(filename, WAVE_SPEC).unwrap()
        });

        let pcm = iq
            .into_iter()
            .low_pass(SAMPLE_RATE, 100_000.0)
            .tuple_windows()
            .map(|(i, q)| {
                let c = i * q.conj();
                let angle = f32::atan2(c.im as f32, c.re as f32);
                angle * 2.0
            });

        let mut err = 0.0;
        let step_by = SAMPLE_RATE as f32 / WAVE_SAMPLE_RATE as f32;

        for sample in pcm
            .low_pass(SAMPLE_RATE, AUDIO_CUTOFF_FREQ)
            .filter(|x| x.abs() < 1.0)
        {
            err += 1.0;
            if err < step_by {
                continue;
            }

            err -= step_by;
            wav.write_sample(sample).unwrap();
        }
    }
}

fn rms(iq: &[Complex<f32>]) -> f32 {
    (iq.iter().map(|c| c.re * c.re + c.im * c.im).sum::<f32>() / iq.len() as f32).sqrt()
}
