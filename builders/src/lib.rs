use caw_core_next::{BufferedSignal, Freq, Never, Signal, SignalCtx, Trigger};
use caw_proc_macros::builder;

pub mod waveform {
    use std::f64::consts::PI;

    pub trait Waveform {
        fn sample(&self, state_01: f64, pulse_width_01: f64) -> f64;

        const PULSE: bool = false;
    }

    pub struct Sine;
    impl Waveform for Sine {
        fn sample(&self, state_01: f64, _pulse_width_01: f64) -> f64 {
            (state_01 * PI * 2.0).sin()
        }
    }

    pub struct Triangle;
    impl Waveform for Triangle {
        fn sample(&self, state_01: f64, _pulse_width_01: f64) -> f64 {
            (((state_01 * 2.0) - 1.0).abs() * 2.0) - 1.0
        }
    }

    pub struct Saw;
    impl Waveform for Saw {
        fn sample(&self, state_01: f64, _pulse_width_01: f64) -> f64 {
            (state_01 * 2.0) - 1.0
        }
    }

    pub struct Pulse;
    impl Waveform for Pulse {
        fn sample(&self, state_01: f64, pulse_width_01: f64) -> f64 {
            if state_01 < pulse_width_01 {
                -1.0
            } else {
                1.0
            }
        }

        const PULSE: bool = true;
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
    waveform: W,
    freq: BufferedSignal<F>,
    pulse_width_01: BufferedSignal<P>,
    reset_offset_01: BufferedSignal<R>,
    reset_trigger: BufferedSignal<T>,
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
    type SampleBuffer = Vec<Self::Item>;

    fn sample_batch(
        &mut self,
        ctx: &SignalCtx,
        n: usize,
        sample_buffer: &mut Self::SampleBuffer,
    ) {
        if W::PULSE {
            self.sample_batch_pulse(ctx, n, sample_buffer);
        } else {
            self.sample_batch_non_pulse(ctx, n, sample_buffer);
        }
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
            waveform,
            freq: freq.buffered(),
            pulse_width_01: pulse_width_01.buffered(),
            reset_offset_01: reset_offset_01.buffered(),
            reset_trigger: reset_trigger.buffered(),
        }
    }

    fn sample_batch_non_pulse(
        &mut self,
        ctx: &SignalCtx,
        n: usize,
        sample_buffer: &mut <Self as Signal>::SampleBuffer,
    ) {
        self.freq.sample_batch(ctx, n);
        self.reset_trigger.sample_batch(ctx, n);
        self.reset_offset_01.sample_batch(ctx, n);
        for ((freq, &reset_trigger), reset_offset_01) in self
            .freq
            .samples()
            .zip(self.reset_trigger.samples())
            .zip(self.reset_offset_01.samples())
        {
            if reset_trigger {
                self.state_01 = *reset_offset_01;
            } else {
                let state_delta = freq.hz() / ctx.sample_rate_hz;
                self.state_01 = (self.state_01 + state_delta).rem_euclid(1.0);
            }
            let sample = self.waveform.sample(self.state_01, 0.0);
            sample_buffer.push(sample);
        }
    }

    fn sample_batch_pulse(
        &mut self,
        ctx: &SignalCtx,
        n: usize,
        sample_buffer: &mut <Self as Signal>::SampleBuffer,
    ) {
        self.freq.sample_batch(ctx, n);
        self.reset_trigger.sample_batch(ctx, n);
        self.reset_offset_01.sample_batch(ctx, n);
        self.pulse_width_01.sample_batch(ctx, n);
        for (((freq, &reset_trigger), reset_offset_01), &pulse_width_01) in self
            .freq
            .samples()
            .zip(self.reset_trigger.samples())
            .zip(self.reset_offset_01.samples())
            .zip(self.pulse_width_01.samples())
        {
            if reset_trigger {
                self.state_01 = *reset_offset_01;
            } else {
                let state_delta = freq.hz() / ctx.sample_rate_hz;
                self.state_01 = (self.state_01 + state_delta).rem_euclid(1.0);
            }
            let sample = self.waveform.sample(self.state_01, pulse_width_01);
            sample_buffer.push(sample);
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
