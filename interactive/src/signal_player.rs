use crate::{
    sample_player::SamplePlayer,
    signal::{Signal, SignalCtx},
};

pub struct SignalPlayer {
    sample_player: SamplePlayer<f32>,
    sample_index: u64,
}

pub trait ToF32 {
    fn to_f32(self) -> f32;
}

impl ToF32 for f32 {
    fn to_f32(self) -> f32 {
        self
    }
}

impl ToF32 for f64 {
    fn to_f32(self) -> f32 {
        self as f32
    }
}

impl SignalPlayer {
    pub fn new() -> anyhow::Result<Self> {
        Ok(Self {
            sample_player: SamplePlayer::new()?,
            sample_index: 0,
        })
    }

    pub fn send_signal<T: Clone + ToF32 + 'static>(&mut self, buffered_signal: &mut Signal<T>) {
        let sample_rate_hz = self.sample_player.sample_rate_hz();
        self.sample_player.play_stream(|| {
            let ctx = SignalCtx {
                sample_index: self.sample_index,
                sample_rate_hz: sample_rate_hz as f64,
            };
            let sample = buffered_signal.sample(&ctx).to_f32();
            self.sample_index += 1;
            sample
        });
    }
}
