use std::f32::consts::PI;

use num_complex::Complex;
use num_traits::Zero;

pub struct LowPassFilter {
    alpha: f32,
    last_value: Complex<f32>,
}

impl LowPassFilter {
    pub fn new(sample_rate: u32, cutoff_freq: f32) -> Self {
        let rc = (cutoff_freq * 2.0 * PI).recip();
        let dt = (sample_rate as f32).recip();
        let alpha = dt / (rc + dt);

        Self {
            alpha,
            last_value: Complex::zero(),
        }
    }

    pub fn prime(&mut self, value: Complex<f32>) {
        self.last_value = value;
    }

    pub fn filter(&mut self, value: Complex<f32>) -> Complex<f32> {
        self.last_value = self.last_value + self.alpha * (value - self.last_value);
        self.last_value
    }
}

pub trait LowPassExt<T> {
    fn low_pass(self, sample_rate: u32, cutoff_freq: f32) -> impl Iterator<Item = T>;
}

impl<Iter> LowPassExt<Complex<f32>> for Iter
where
    Iter: Iterator<Item = Complex<f32>>,
{
    fn low_pass(
        mut self,
        sample_rate: u32,
        cutoff_freq: f32,
    ) -> impl Iterator<Item = Complex<f32>> {
        let mut filter = LowPassFilter::new(sample_rate, cutoff_freq);
        filter.prime(self.next().unwrap());
        self.scan(filter, |filter, value| Some(filter.filter(value)))
    }
}

impl<Iter> LowPassExt<f32> for Iter
where
    Iter: Iterator<Item = f32>,
{
    fn low_pass(mut self, sample_rate: u32, cutoff_freq: f32) -> impl Iterator<Item = f32> {
        let mut filter = LowPassFilter::new(sample_rate, cutoff_freq);
        filter.prime(Complex::new(self.next().unwrap(), 0.0));
        self.map(move |value| filter.filter(Complex::new(value, 0.0)).re)
    }
}
