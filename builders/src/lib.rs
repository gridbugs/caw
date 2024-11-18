use caw_core_next::{Buf, Never, Sig, SigCtx, SigT, TrigT};
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

pub use waveform::*;

struct Oscillator<W, F, P, R, T>
where
    W: Waveform,
    F: SigT<Item = f32>,
    P: SigT<Item = f32>,
    R: SigT<Item = f32>,
    T: TrigT,
{
    first_frame: bool,
    state_01: f32,
    waveform: W,
    freq: F,
    pulse_width_01: P,
    reset_offset_01: R,
    reset_trigger: T,
    buf: Vec<f32>,
}

impl<W, F, P, R, T> SigT for Oscillator<W, F, P, R, T>
where
    W: Waveform,
    F: SigT<Item = f32>,
    P: SigT<Item = f32>,
    R: SigT<Item = f32>,
    T: TrigT,
{
    type Item = f32;

    fn sample(&mut self, ctx: &SigCtx) -> impl Buf<Self::Item> {
        if W::PULSE {
            self.sample_batch_pulse(ctx);
        } else {
            self.sample_batch_non_pulse(ctx);
        }
        &self.buf
    }
}

impl<W, F, P, R, T> Oscillator<W, F, P, R, T>
where
    W: Waveform,
    F: SigT<Item = f32>,
    P: SigT<Item = f32>,
    R: SigT<Item = f32>,
    T: TrigT,
{
    fn new(
        waveform: W,
        freq: F,
        pulse_width_01: P,
        reset_offset_01: R,
        reset_trigger: T,
    ) -> Sig<Self> {
        Sig(Self {
            first_frame: true,
            state_01: 0.0,
            waveform,
            freq,
            pulse_width_01,
            reset_offset_01,
            reset_trigger,
            buf: Vec::new(),
        })
    }

    fn sample_batch_non_pulse(&mut self, ctx: &SigCtx) {
        let buf_freq = self.freq.sample(ctx);
        let buf_reset_trigger = self.reset_trigger.sample(ctx);
        let buf_reset_offset_01 = self.reset_offset_01.sample(ctx);
        self.buf.clear();
        for ((freq, &reset_trigger), reset_offset_01) in buf_freq
            .iter()
            .zip(buf_reset_trigger.iter())
            .zip(buf_reset_offset_01.iter())
        {
            if reset_trigger || self.first_frame {
                self.first_frame = false;
                self.state_01 = *reset_offset_01;
            } else {
                let state_delta = freq / ctx.sample_rate_hz;
                self.state_01 = self.state_01 + state_delta;
                self.state_01 = self.state_01 - (self.state_01 - 0.5).round();
            }
            let sample = self.waveform.sample(self.state_01, 0.0);
            self.buf.push(sample);
        }
    }

    // The pulse wave oscillator is specialized because in all other waveforms there's no need to
    // compute the values of the pulse width signal.
    fn sample_batch_pulse(&mut self, ctx: &SigCtx) {
        let buf_freq = self.freq.sample(ctx);
        let buf_reset_trigger = self.reset_trigger.sample(ctx);
        let buf_reset_offset_01 = self.reset_offset_01.sample(ctx);
        let buf_pulse_width_01 = self.pulse_width_01.sample(ctx);
        self.buf.clear();
        for (((freq, &reset_trigger), reset_offset_01), &pulse_width_01) in
            buf_freq
                .iter()
                .zip(buf_reset_trigger.iter())
                .zip(buf_reset_offset_01.iter())
                .zip(buf_pulse_width_01.iter())
        {
            if reset_trigger || self.first_frame {
                self.first_frame = false;
                self.state_01 = *reset_offset_01;
            } else {
                let state_delta = freq / ctx.sample_rate_hz;
                self.state_01 = (self.state_01 + state_delta).rem_euclid(1.0);
            }
            let sample = self.waveform.sample(self.state_01, pulse_width_01);
            self.buf.push(sample);
        }
    }
}

builder!(
    #[allow(unused)]
    #[constructor = "oscillator"]
    #[constructor_doc = "A signal which oscillates with a given waveform at a given frequency."]
    #[build_fn = "Oscillator::new"]
    #[build_ty = "Sig<impl SigT<Item = f32>>"]
    #[generic_setter_type_name = "X"]
    pub struct OscillatorBuilder {
        #[generic_with_constraint = "Waveform"]
        #[generic_name = "W"]
        waveform: _,
        #[generic_with_constraint = "SigT<Item = f32>"]
        #[generic_name = "F"]
        freq: _,
        #[generic_with_constraint = "SigT<Item = f32>"]
        #[default = 0.5]
        #[generic_name = "P"]
        pulse_width_01: f32,
        #[generic_with_constraint = "SigT<Item = f32>"]
        #[default = 0.0]
        #[generic_name = "R"]
        reset_offset_01: f32,
        #[generic_with_constraint = "TrigT"]
        #[default = Never]
        #[generic_name = "T"]
        reset_trigger: Never,
    }
);
