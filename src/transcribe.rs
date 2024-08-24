use anyhow::Result;
use whisper_rs::{self, FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters};

pub struct Transcriber {
    model: WhisperContext,
}

impl Transcriber {
    pub fn new(model: &str) -> Result<Transcriber> {
        let model =
            WhisperContext::new_with_params(model, WhisperContextParameters::default()).unwrap();
        Ok(Self { model })
    }

    /// Audio must be 16kHz mono
    pub fn transcribe(&mut self, audio: &[f32]) -> Result<String> {
        let mut state = self.model.create_state()?;
        let params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });

        state.full(params, audio)?;

        let mut out = String::new();
        for i in 0..state.full_n_segments()? {
            let segment = state.full_get_segment_text(i)?;
            out.push_str(&segment);
        }

        Ok(out)
    }
}
