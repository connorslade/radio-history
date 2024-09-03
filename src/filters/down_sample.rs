pub struct DownSample<Iter: Iterator> {
    step_by: f32,

    error: f32,
    inner: Iter,
}

impl<Iter: Iterator> DownSample<Iter> {
    pub fn new(inner: Iter, in_sample_rate: u32, out_sample_rate: u32) -> Self {
        Self {
            step_by: in_sample_rate as f32 / out_sample_rate as f32,
            error: 0.0,
            inner,
        }
    }
}

impl<Iter: Iterator> Iterator for DownSample<Iter> {
    type Item = Iter::Item;

    fn next(&mut self) -> Option<Self::Item> {
        for next in &mut self.inner {
            self.error += 1.0;
            if self.error >= self.step_by {
                self.error -= self.step_by;
                return Some(next);
            }
        }

        None
    }
}

pub trait DownSampleExt: Iterator {
    fn down_sample(self, in_sample_rate: u32, out_sample_rate: u32) -> DownSample<Self>
    where
        Self: Sized,
    {
        DownSample::new(self, in_sample_rate, out_sample_rate)
    }
}

impl<Iter: Iterator> DownSampleExt for Iter {}
