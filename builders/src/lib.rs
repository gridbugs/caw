use caw_core_next::{Freq, Never, Sig, SigBuf, SigCtx, Trig};
use caw_proc_macros::builder;

pub mod waveform {
    use std::f32::consts::PI;

    pub trait Waveform {
        fn sample(&self, state_01: f32, pulse_width_01: f32) -> f32;

        const PULSE: bool = false;
    }

    pub struct Sine;
    impl Waveform for Sine {
        fn sample(&self, state_01: f32, _pulse_width_01: f32) -> f32 {
            (state_01 * PI * 2.0).sin()
        }
    }

    pub struct Triangle;
    impl Waveform for Triangle {
        fn sample(&self, state_01: f32, _pulse_width_01: f32) -> f32 {
            (((state_01 * 2.0) - 1.0).abs() * 2.0) - 1.0
        }
    }

    pub struct Saw;
    impl Waveform for Saw {
        fn sample(&self, state_01: f32, _pulse_width_01: f32) -> f32 {
            (state_01 * 2.0) - 1.0
        }
    }

    pub struct Pulse;
    impl Waveform for Pulse {
        fn sample(&self, state_01: f32, pulse_width_01: f32) -> f32 {
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
    F: Sig<Item = Freq>,
    P: Sig<Item = f32>,
    R: Sig<Item = f32>,
    T: Trig,
{
    first_frame: bool,
    state_01: f32,
    waveform: W,
    freq: SigBuf<F>,
    pulse_width_01: SigBuf<P>,
    reset_offset_01: SigBuf<R>,
    reset_trigger: SigBuf<T>,
}

impl<W, F, P, R, T> Sig for Oscillator<W, F, P, R, T>
where
    W: Waveform,
    F: Sig<Item = Freq>,
    P: Sig<Item = f32>,
    R: Sig<Item = f32>,
    T: Trig,
{
    type Item = f32;
    type Buf = Vec<Self::Item>;

    fn sample_batch(&mut self, ctx: &SigCtx, sample_buffer: &mut Self::Buf) {
        if W::PULSE {
            self.sample_batch_pulse(ctx, sample_buffer);
        } else {
            self.sample_batch_non_pulse(ctx, sample_buffer);
        }
    }
}

impl<W, F, P, R, T> Oscillator<W, F, P, R, T>
where
    W: Waveform,
    F: Sig<Item = Freq>,
    P: Sig<Item = f32>,
    R: Sig<Item = f32>,
    T: Trig,
{
    fn new(
        waveform: W,
        freq: F,
        pulse_width_01: P,
        reset_offset_01: R,
        reset_trigger: T,
    ) -> Self {
        Self {
            first_frame: true,
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
        ctx: &SigCtx,
        sample_buffer: &mut <Self as Sig>::Buf,
    ) {
        self.freq.sample_batch(ctx);
        self.reset_trigger.sample_batch(ctx);
        self.reset_offset_01.sample_batch(ctx);
        for ((freq, &reset_trigger), reset_offset_01) in self
            .freq
            .samples()
            .zip(self.reset_trigger.samples())
            .zip(self.reset_offset_01.samples())
        {
            if reset_trigger || self.first_frame {
                self.first_frame = false;
                self.state_01 = *reset_offset_01;
            } else {
                let state_delta = freq.hz() / ctx.sample_rate_hz;
                self.state_01 = self.state_01 + state_delta;
                self.state_01 = self.state_01 - (self.state_01 - 0.5).round();
            }
            let sample = self.waveform.sample(self.state_01, 0.0);
            sample_buffer.push(sample);
        }
    }

    // The pulse wave oscillator is specialized because in all other waveforms there's no need to
    // compute the values of the pulse width signal.
    fn sample_batch_pulse(
        &mut self,
        ctx: &SigCtx,
        sample_buffer: &mut <Self as Sig>::Buf,
    ) {
        self.freq.sample_batch(ctx);
        self.reset_trigger.sample_batch(ctx);
        self.reset_offset_01.sample_batch(ctx);
        self.pulse_width_01.sample_batch(ctx);
        for (((freq, &reset_trigger), reset_offset_01), &pulse_width_01) in self
            .freq
            .samples()
            .zip(self.reset_trigger.samples())
            .zip(self.reset_offset_01.samples())
            .zip(self.pulse_width_01.samples())
        {
            if reset_trigger || self.first_frame {
                self.first_frame = false;
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
    #[build_ty = "impl Sig<Item = f32, Buf = Vec<f32>>"]
    #[generic_setter_type_name = "X"]
    pub struct OscillatorBuilder {
        #[generic_with_constraint = "Waveform"]
        #[generic_name = "W"]
        waveform: _,
        #[generic_with_constraint = "Sig<Item = Freq>"]
        #[generic_name = "F"]
        freq: _,
        #[generic_with_constraint = "Sig<Item = f32>"]
        #[default = 0.5]
        #[generic_name = "P"]
        pulse_width_01: f32,
        #[generic_with_constraint = "Sig<Item = f32>"]
        #[default = 0.0]
        #[generic_name = "R"]
        reset_offset_01: f32,
        #[generic_with_constraint = "Trig"]
        #[default = Never]
        #[generic_name = "T"]
        reset_trigger: Never,
    }
);
