use std::f32::consts::TAU;

use num_complex::Complex;

pub struct OffsetFilter {
    offset_angle: f32,

    sample: u32,
    sample_rate: u32,
}

impl OffsetFilter {
    pub fn new(offset_freq: f32, sample_rate: u32) -> Self {
        Self {
            offset_angle: TAU * offset_freq,
            sample: 0,
            sample_rate,
        }
    }

    pub fn filter(&mut self, iq: Complex<f32>) -> Complex<f32> {
        let t = self.sample as f32 / self.sample_rate as f32;
        let angle = self.offset_angle * t * Complex::i();

        self.sample += 1;
        iq * angle.exp()
    }
}

pub trait OffsetExt {
    fn offset(self, offset_freq: f32, sample_rate: u32) -> impl Iterator<Item = Complex<f32>>;
}

impl<Iter> OffsetExt for Iter
where
    Iter: Iterator<Item = Complex<f32>>,
{
    fn offset(self, offset_freq: f32, sample_rate: u32) -> impl Iterator<Item = Complex<f32>> {
        let mut filter = OffsetFilter::new(offset_freq, sample_rate);
        self.map(move |iq| filter.filter(iq))
    }
}
