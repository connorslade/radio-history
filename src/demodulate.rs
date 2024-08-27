use std::{f32::consts::PI, iter};

use itertools::Itertools;
use num_complex::Complex;
use num_traits::Zero;

use crate::{
    consts::{AUDIO_CUTOFF_FREQ, SAMPLE_RATE, WAVE_SAMPLE_RATE},
    filters::{down_sample::DownSampleExt, low_pass::LowPassExt},
};

pub struct Demodulator {
    iq: Vec<Complex<f32>>,
    last_sample: Complex<f32>,
}

impl Demodulator {
    pub fn empty() -> Self {
        Self {
            iq: Vec::new(),
            last_sample: Complex::zero(),
        }
    }

    pub fn replace(&mut self, data: &[u8]) {
        self.iq = data
            .chunks_exact(2)
            .map(|chunk| Complex::new(chunk[0] as f32 / 127.5 - 1.0, chunk[1] as f32 / 127.5 - 1.0))
            .collect::<Vec<_>>();
    }

    pub fn rms(&self) -> f32 {
        (self
            .iq
            .iter()
            .map(|c| c.re * c.re + c.im * c.im)
            .sum::<f32>()
            / self.iq.len() as f32)
            .sqrt()
    }

    pub fn iq(&self) -> &[Complex<f32>] {
        &self.iq
    }

    pub fn to_audio(&mut self, gain: f32) -> Vec<f32> {
        let mut audio = iter::once(self.last_sample)
            .chain(self.iq.iter().copied())
            .low_pass(SAMPLE_RATE, 20_000.0)
            .tuple_windows()
            .map(|(a, b)| {
                self.last_sample = b;

                let mut angle = b.arg() - a.arg();
                if angle > PI {
                    angle -= 2.0 * PI;
                } else if angle < -PI {
                    angle += 2.0 * PI;
                }

                angle * gain
            })
            .low_pass(SAMPLE_RATE, AUDIO_CUTOFF_FREQ)
            .down_sample(SAMPLE_RATE, WAVE_SAMPLE_RATE)
            .collect::<Vec<_>>();

        let mean = audio.iter().sum::<f32>() / audio.len() as f32;
        audio.iter_mut().for_each(|v| *v -= mean);

        audio
    }
}
