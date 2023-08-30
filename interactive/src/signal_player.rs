use crate::{
    sample_player::SamplePlayer,
    signal::{Signal, SignalCtx},
};

pub struct SignalPlayer {
    sample_player: SamplePlayer<f32>,
    sample_index: u64,
}

impl SignalPlayer {
    pub fn new() -> anyhow::Result<Self> {
        Ok(Self {
            sample_player: SamplePlayer::new()?,
            sample_index: 0,
        })
    }

    pub fn send_signal(&mut self, buffered_signal: &mut Signal<f32>) {
        let sample_rate_hz = self.sample_player.sample_rate_hz();
        self.sample_player.play_stream(|| {
            let ctx = SignalCtx {
                sample_index: self.sample_index,
                sample_rate_hz: sample_rate_hz as f64,
            };
            let sample = buffered_signal.sample(&ctx);
            self.sample_index += 1;
            sample
        });
    }
}
