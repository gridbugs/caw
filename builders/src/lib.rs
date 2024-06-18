use caw_core_next::{Freq, Never, Signal, Trigger};
use caw_proc_macros::builder;

pub mod waveform {
    use std::f64::consts::PI;

    pub trait Waveform {
        /// Note that the `pulse_width_01` is a function as most
        /// waveforms ignore it, and this lets us avoid sampling the
        /// pulse_width_01 signal on each frame of oscillators that
        /// don't need it.
        fn sample(
            &self,
            state_01: f64,
            pulse_width_01: impl FnMut() -> f64,
        ) -> f64;
    }

    pub struct Sine;
    impl Waveform for Sine {
        fn sample(
            &self,
            state_01: f64,
            _pulse_width_01: impl FnMut() -> f64,
        ) -> f64 {
            (state_01 * PI * 2.0).sin()
        }
    }

    pub struct Triangle;
    impl Waveform for Triangle {
        fn sample(
            &self,
            state_01: f64,
            _pulse_width_01: impl FnMut() -> f64,
        ) -> f64 {
            (((state_01 * 2.0) - 1.0).abs() * 2.0) - 1.0
        }
    }

    pub struct Saw;
    impl Waveform for Saw {
        fn sample(
            &self,
            state_01: f64,
            _pulse_width_01: impl FnMut() -> f64,
        ) -> f64 {
            (state_01 * 2.0) - 1.0
        }
    }

    pub struct Pulse;
    impl Waveform for Pulse {
        fn sample(
            &self,
            state_01: f64,
            mut pulse_width_01: impl FnMut() -> f64,
        ) -> f64 {
            if state_01 < pulse_width_01() {
                -1.0
            } else {
                1.0
            }
        }
    }
}

pub use waveform::Waveform;

struct Oscillator<W, F, P, R, T>
where
    W: Waveform,
    F: Signal<Item = Freq>,
    P: Signal<Item = f64>,
    R: Signal<Item = f64>,
    T: Trigger,
{
    state_01: f64,
    prev_sample_index: u64,
    waveform: W,
    freq: F,
    pulse_width_01: P,
    reset_offset_01: R,
    reset_trigger: T,
}

impl<W, F, P, R, T> Signal for Oscillator<W, F, P, R, T>
where
    W: Waveform,
    F: Signal<Item = Freq>,
    P: Signal<Item = f64>,
    R: Signal<Item = f64>,
    T: Trigger,
{
    type Item = f64;

    fn sample(&mut self, ctx: &caw_core_next::SignalCtx) -> Self::Item {
        if self.reset_trigger.sample(ctx) {
            self.state_01 = self.reset_offset_01.sample(ctx);
        } else {
            let sample_index_delta = ctx.sample_index - self.prev_sample_index;
            let state_delta = (sample_index_delta as f64
                * self.freq.sample(ctx).hz())
                / ctx.sample_rate_hz;
            self.state_01 = (self.state_01 + state_delta).rem_euclid(1.0);
        }
        self.waveform
            .sample(self.state_01, || self.pulse_width_01.sample(ctx))
    }
}

impl<W, F, P, R, T> Oscillator<W, F, P, R, T>
where
    W: Waveform,
    F: Signal<Item = Freq>,
    P: Signal<Item = f64>,
    R: Signal<Item = f64>,
    T: Trigger,
{
    fn new(
        waveform: W,
        freq: F,
        pulse_width_01: P,
        reset_offset_01: R,
        reset_trigger: T,
    ) -> Self {
        Self {
            state_01: 0.0,
            prev_sample_index: 0,
            waveform,
            freq,
            pulse_width_01,
            reset_offset_01,
            reset_trigger,
        }
    }
}

builder!(
    #[allow(unused)]
    #[constructor = "oscillator"]
    #[constructor_doc = "A signal which oscillates with a given waveform at a given frequency."]
    #[build_fn = "Oscillator::new"]
    #[build_ty = "impl Signal<Item = f64>"]
    #[generic_setter_type_name = "X"]
    pub struct OscillatorBuilder {
        #[generic_with_constraint = "Waveform"]
        #[generic_name = "W"]
        waveform: _,
        #[generic_with_constraint = "Signal<Item = Freq>"]
        #[generic_name = "F"]
        freq: _,
        #[generic_with_constraint = "Signal<Item = f64>"]
        #[default = 0.5]
        #[generic_name = "P"]
        pulse_width_01: f64,
        #[generic_with_constraint = "Signal<Item = f64>"]
        #[default = 0.0]
        #[generic_name = "R"]
        reset_offset_01: f64,
        #[generic_with_constraint = "Trigger"]
        #[default = Never]
        #[generic_name = "T"]
        reset_trigger: Never,
    }
);
