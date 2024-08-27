use hound::{SampleFormat, WavSpec};

pub const BUFFER_SIZE: usize = 16_384;
pub const SAMPLE_RATE: u32 = 250_000;
pub const AUDIO_CUTOFF_FREQ: f32 = 15_000.0;

pub const SQUELCH: f32 = 0.05;

pub const WAVE_SAMPLE_RATE: u32 = 44_100;
pub const WAVE_SPEC: WavSpec = WavSpec {
    channels: 1,
    sample_rate: WAVE_SAMPLE_RATE,
    bits_per_sample: 8,
    sample_format: SampleFormat::Int,
};
