use std::io::{stdout, Write};

use hound::{SampleFormat, WavSpec, WavWriter};

use itertools::Itertools;
use num_complex::Complex;

mod low_pass;
use low_pass::LowPassExt;

const BUFFER_SIZE: usize = 16_384;
const SAMPLE_RATE: u32 = 250_000;

fn main() {
    let mut device = rtlsdr::open(0).unwrap();

    // device.set_center_freq(156_450_000).unwrap();
    device.set_center_freq(99_100_000).unwrap();
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

    let mut time = 0.0;
    loop {
        print!("\r{time:.1}s");
        stdout().flush().unwrap();

        time += BUFFER_SIZE as f32 / SAMPLE_RATE as f32 / 2.0;

        let data = device.read_sync(BUFFER_SIZE).unwrap();
        let iq = data.chunks_exact(2).map(|chunk| {
            Complex::new(chunk[0] as f32 / 128.0 - 1.0, chunk[1] as f32 / 128.0 - 1.0)
        });
        let pcm = iq
            .low_pass(SAMPLE_RATE, 100_000.0)
            .tuple_windows()
            .map(|(i, q)| {
                let c = i * q.conj();
                let angle = f32::atan2(c.im as f32, c.re as f32);
                angle
            });

        for sample in pcm {
            wav.write_sample(sample).unwrap();
        }
    }

    wav.finalize().unwrap();
}
