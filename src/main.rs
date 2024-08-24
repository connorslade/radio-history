use std::io::{stdout, Write};

use hound::{SampleFormat, WavSpec, WavWriter};

use itertools::Itertools;
use num_complex::{Complex, ComplexFloat};

mod low_pass;
use low_pass::LowPassExt;

const BUFFER_SIZE: usize = 16_384;
const SAMPLE_RATE: u32 = 250_000;

const SQUELCH: f32 = 0.6;

fn main() {
    let mut device = rtlsdr::open(0).unwrap();

    device.set_center_freq(156_450_000).unwrap();
    device.set_sample_rate(SAMPLE_RATE).unwrap();
    device.reset_buffer().unwrap();

    let mut wav = WavWriter::create(
        "out.wav",
        WavSpec {
            channels: 1,
            // sample_rate: 44100,
            sample_rate: SAMPLE_RATE,
            bits_per_sample: 32,
            sample_format: SampleFormat::Float,
        },
    )
    .unwrap();

    let mut had_message = false;
    let mut time = 0.0;
    loop {
        print!("\r{time:.1}s");
        stdout().flush().unwrap();

        time += BUFFER_SIZE as f32 / SAMPLE_RATE as f32 / 2.0;

        let data = device.read_sync(BUFFER_SIZE).unwrap();
        let iq = data
            .chunks_exact(2)
            .map(|chunk| Complex::new(chunk[0] as f32 / 128.0 - 1.0, chunk[1] as f32 / 128.0 - 1.0))
            .collect::<Vec<_>>();

        let rms = rms(&iq);
        println!("rms: {}", rms);
        if rms < SQUELCH {
            if had_message {
                break;
            }
            continue;
        }

        had_message = true;
        let pcm = iq
            .into_iter()
            .low_pass(SAMPLE_RATE, 100_000.0)
            .tuple_windows()
            .map(|(i, q)| {
                let c = i * q.conj();
                let angle = f32::atan2(c.im as f32, c.re as f32);
                angle * 2.0
            });

        for sample in pcm.filter(|x| x.abs() < 1.0) {
            wav.write_sample(sample).unwrap();
        }
    }

    wav.finalize().unwrap();
}

fn rms(iq: &[Complex<f32>]) -> f32 {
    (iq.iter().map(|c| c.re * c.re + c.im * c.im).sum::<f32>() / iq.len() as f32).sqrt()
}
