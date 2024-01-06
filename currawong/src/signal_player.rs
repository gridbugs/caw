use crate::{
    sample_player::SamplePlayer,
    signal::{Signal, SignalCtx},
};

pub struct SignalPlayer {
    sample_player: SamplePlayer,
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

    pub fn new_with_downsample(downsample: u32) -> anyhow::Result<Self> {
        Ok(Self {
            sample_player: SamplePlayer::new_with_downsample(downsample)?,
            sample_index: 0,
        })
    }

    pub fn send_signal_with_callback<T: Copy + Default + ToF32 + 'static, F: FnMut(f32)>(
        &mut self,
        signal: &mut Signal<T>,
        mut f: F,
    ) {
        let sample_rate_hz = self.sample_player.sample_rate_hz();
        self.sample_player.play_stream(|| {
            let ctx = SignalCtx {
                sample_index: self.sample_index,
                sample_rate_hz: sample_rate_hz as f64,
            };
            let sample = signal.sample(&ctx).to_f32();
            f(sample);
            self.sample_index += 1;
            sample
        });
    }

    pub fn send_signal<T: Copy + Default + ToF32 + 'static>(&mut self, signal: &mut Signal<T>) {
        self.send_signal_with_callback(signal, |_| ());
    }

    #[cfg(not(feature = "web"))]
    pub fn play_sample_forever<T: Copy + Default + ToF32 + 'static>(
        &mut self,
        mut signal: Signal<T>,
    ) -> ! {
        use std::{thread, time::Duration};
        const PERIOD: Duration = Duration::from_millis(16);
        loop {
            self.send_signal(&mut signal);
            thread::sleep(PERIOD);
        }
    }

    pub fn set_volume(&self, volume: f32) {
        self.sample_player.set_volume(volume);
    }
}
