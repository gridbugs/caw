use crate::signal::{const_, Sf64, Sfreq, Signal, Trigger};
use std::{cell::Cell, f64::consts::PI};

#[derive(Default, Clone, Copy, Debug)]
pub enum Waveform {
    #[default]
    Sine,
    Saw,
    Triangle,
    Pulse,
}

impl From<Waveform> for Signal<Waveform> {
    fn from(value: Waveform) -> Self {
        const_(value)
    }
}

pub struct Oscillator {
    pub waveform: Signal<Waveform>,
    pub freq: Sfreq,
    pub pulse_width_01: Sf64,
    pub reset_trigger: Trigger,
    pub reset_offset_01: Sf64,
    pub hard_sync: Sf64,
}

impl Oscillator {
    pub fn signal(self) -> Sf64 {
        let state_opt = Cell::new(None);
        let prev_sample_index = Cell::new(0);
        let prev_hard_sync_sample: Cell<f64> = Cell::new(0.0);
        Signal::from_fn(move |ctx| {
            let sample_index_delta = ctx.sample_index - prev_sample_index.get();
            prev_sample_index.set(ctx.sample_index);
            if sample_index_delta == 0 {
                return 0.0;
            }
            let hard_sync_sample = self.hard_sync.sample(ctx);
            let state = if hard_sync_sample > 0.0 && prev_hard_sync_sample.get() <= 0.0 {
                self.reset_offset_01.sample(ctx)
            } else {
                match state_opt.get() {
                    None => self.reset_offset_01.sample(ctx),
                    Some(state) => {
                        if self.reset_trigger.sample(ctx) {
                            self.reset_offset_01.sample(ctx)
                        } else {
                            state
                        }
                    }
                }
            };
            prev_hard_sync_sample.set(hard_sync_sample);
            let state_delta =
                (sample_index_delta as f64 * self.freq.sample(ctx).hz()) / ctx.sample_rate_hz;
            let try_state = (state + state_delta).rem_euclid(1.0);
            let state = if try_state.is_nan() { state } else { try_state };
            state_opt.set(Some(state));
            match self.waveform.sample(ctx) {
                Waveform::Sine => (state * PI * 2.0).sin(),
                Waveform::Saw => (state * 2.0) - 1.0,
                Waveform::Triangle => (((state * 2.0) - 1.0).abs() * 2.0) - 1.0,
                Waveform::Pulse => {
                    if state < self.pulse_width_01.sample(ctx) {
                        -1.0
                    } else {
                        1.0
                    }
                }
            }
        })
    }
}
