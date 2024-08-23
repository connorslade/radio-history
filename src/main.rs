use hound::{SampleFormat, WavSpec, WavWriter};
use num_complex::Complex;

fn main() {
    let mut device = rtlsdr::open(0).unwrap();

    device.set_center_freq(99_100_000).unwrap();
    device.set_sample_rate(250_000).unwrap();
    device.reset_buffer().unwrap();

    let mut wav = WavWriter::create(
        "out-nofilter.wav",
        WavSpec {
            channels: 1,
            // sample_rate: 44100,
            sample_rate: 250_000,
            bits_per_sample: 32,
            sample_format: SampleFormat::Float,
        },
    )
    .unwrap();

    for i in 0..100 {
        println!("{i}/100 - {}%", i + 1);

        let data = device.read_sync(16 * 16384).unwrap();
        let iq = data
            .chunks_exact(2)
            .map(|chunk| Complex::new(chunk[0] as f32 / 128.0 - 1.0, chunk[1] as f32 / 128.0 - 1.0))
            .collect::<Vec<_>>();
        let pcm = iq.windows(2).map(|window| {
            let c = window[0] * window[1].conj();
            let angle = f32::atan2(c.im as f32, c.re as f32);
            angle * 0.4
        });

        // let rc = (150_000.0 * 2.0 * PI).recip();
        // let dt = (250_000_f32).recip();
        // let alpha = dt / (rc + dt);

        // let mut filtered = vec![0.0; data.len() / 2];
        // filtered[0] = pcm.next().unwrap();
        // for (i, sample) in pcm.enumerate() {
        //     filtered[i + 1] = filtered[i] + alpha * (sample - filtered[i]);
        // }

        // for sample in filtered {
        //     wav.write_sample(sample).unwrap();
        // }

        for sample in pcm {
            wav.write_sample(sample).unwrap();
        }
    }

    wav.finalize().unwrap();
}
