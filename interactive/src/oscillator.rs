use crate::signal::{const_, sfreq_hz, sfreq_s, Sf64, Sfreq, Signal, Trigger};
use rand::{rngs::StdRng, Rng, SeedableRng};
use std::f64::consts::PI;

#[derive(Clone, Copy, Debug)]
pub enum Waveform {
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
    pub frequency: Sfreq,
    pub pulse_width_01: Sf64,
    pub reset_trigger: Trigger,
    pub reset_offset_01: Sf64,
}

pub struct OscillatorBuilder {
    waveform: Signal<Waveform>,
    frequency: Sfreq,
    pulse_width_01: Option<Sf64>,
    reset_trigger: Option<Trigger>,
    reset_offset_01: Option<Sf64>,
}

impl OscillatorBuilder {
    pub fn new(waveform: impl Into<Signal<Waveform>>, frequency: impl Into<Sfreq>) -> Self {
        Self {
            waveform: waveform.into(),
            frequency: frequency.into(),
            pulse_width_01: None,
            reset_trigger: None,
            reset_offset_01: None,
        }
    }

    pub fn pulse_width_01(mut self, pulse_width_01: impl Into<Sf64>) -> Self {
        self.pulse_width_01 = Some(pulse_width_01.into());
        self
    }

    pub fn reset_trigger(mut self, reset_trigger: impl Into<Trigger>) -> Self {
        self.reset_trigger = Some(reset_trigger.into());
        self
    }

    pub fn reset_offset_01(mut self, reset_offset_01: impl Into<Sf64>) -> Self {
        self.reset_offset_01 = Some(reset_offset_01.into());
        self
    }

    pub fn build(self) -> Oscillator {
        Oscillator {
            waveform: self.waveform,
            frequency: self.frequency,
            pulse_width_01: self.pulse_width_01.unwrap_or_else(|| const_(0.5)),
            reset_trigger: self.reset_trigger.unwrap_or_else(|| Trigger::never()),
            reset_offset_01: self.reset_offset_01.unwrap_or_else(|| const_(0.0)),
        }
    }

    pub fn build_signal(self) -> Sf64 {
        self.build().signal()
    }
}

impl Oscillator {
    pub fn builder(
        waveform: impl Into<Signal<Waveform>>,
        frequency: impl Into<Sfreq>,
    ) -> OscillatorBuilder {
        OscillatorBuilder::new(waveform, frequency)
    }

    pub fn builder_hz(
        waveform: impl Into<Signal<Waveform>>,
        freq_hz: impl Into<Sf64>,
    ) -> OscillatorBuilder {
        Self::builder(waveform, sfreq_hz(freq_hz))
    }

    pub fn builder_s(
        waveform: impl Into<Signal<Waveform>>,
        freq_s: impl Into<Sf64>,
    ) -> OscillatorBuilder {
        Self::builder(waveform, sfreq_s(freq_s))
    }

    pub fn signal(mut self) -> Sf64 {
        let mut state_opt = None;
        Signal::from_fn(move |ctx| {
            let state = match state_opt {
                None => self.reset_offset_01.sample(ctx),
                Some(state) => {
                    if self.reset_trigger.sample(ctx) {
                        self.reset_offset_01.sample(ctx)
                    } else {
                        state
                    }
                }
            };
            let state_delta = self.frequency.sample(ctx).hz() / ctx.sample_rate_hz;
            let state = (state + state_delta).rem_euclid(1.0);
            state_opt = Some(state);
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

pub fn noise() -> Sf64 {
    let mut rng = StdRng::from_entropy();
    Signal::from_fn(move |_ctx| (rng.gen::<f64>() * 2.0) - 1.0)
}
