use caw_builder_proc_macros::builder;
use caw_core::{Buf, Sig, SigCtx, SigT};

pub mod waveform {
    use std::f32::consts::PI;

    pub trait Waveform: Copy {
        fn sample(&self, state_01: f32, pulse_width_01: f32) -> f32;

        const PULSE: bool = false;
    }

    #[derive(Clone, Copy)]
    pub struct Sine;
    impl Waveform for Sine {
        fn sample(&self, state_01: f32, _pulse_width_01: f32) -> f32 {
            (state_01 * PI * 2.0).sin()
        }
    }

    #[derive(Clone, Copy)]
    pub struct Triangle;
    impl Waveform for Triangle {
        fn sample(&self, state_01: f32, _pulse_width_01: f32) -> f32 {
            (((state_01 * 2.0) - 1.0).abs() * 2.0) - 1.0
        }
    }

    #[derive(Clone, Copy)]
    pub struct Saw;
    impl Waveform for Saw {
        fn sample(&self, state_01: f32, _pulse_width_01: f32) -> f32 {
            (state_01 * 2.0) - 1.0
        }
    }

    #[derive(Clone, Copy)]
    pub struct Pulse;
    impl Waveform for Pulse {
        fn sample(&self, state_01: f32, pulse_width_01: f32) -> f32 {
            if state_01 < pulse_width_01 { -1.0 } else { 1.0 }
        }

        const PULSE: bool = true;
    }
}

use crate::{Pulse, Saw, Sine, Triangle};
pub use waveform::Waveform;

pub struct Oscillator<W, F, P, R, T>
where
    W: Waveform,
    F: SigT<Item = f32>,
    P: SigT<Item = f32>,
    R: SigT<Item = f32>,
    T: SigT<Item = bool>,
{
    first_frame: bool,
    state_01: f32,
    waveform: W,
    freq_hz: F,
    pulse_width_01: P,
    reset_offset_01: R,
    reset_trig: T,
    buf: Vec<f32>,
}

impl<W, F, P, R, T> SigT for Oscillator<W, F, P, R, T>
where
    W: Waveform,
    F: SigT<Item = f32>,
    P: SigT<Item = f32>,
    R: SigT<Item = f32>,
    T: SigT<Item = bool>,
{
    type Item = f32;

    fn sample(&mut self, ctx: &SigCtx) -> impl Buf<Self::Item> {
        if W::PULSE {
            self.sample_pulse(ctx);
        } else {
            self.sample_non_pulse(ctx);
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
    T: SigT<Item = bool>,
{
    fn new(
        waveform: W,
        freq_hz: F,
        pulse_width_01: P,
        reset_offset_01: R,
        reset_trig: T,
    ) -> Sig<Self> {
        Sig(Self {
            first_frame: true,
            state_01: 0.0,
            waveform,
            freq_hz,
            pulse_width_01,
            reset_offset_01,
            reset_trig,
            buf: Vec::new(),
        })
    }

    fn sample_non_pulse(&mut self, ctx: &SigCtx) {
        let buf_freq_hz = self.freq_hz.sample(ctx);
        let buf_reset_trig = self.reset_trig.sample(ctx);
        let buf_reset_offset_01 = self.reset_offset_01.sample(ctx);
        self.buf.clear();
        for ((freq_hz, reset_trig), reset_offset_01) in buf_freq_hz
            .iter()
            .zip(buf_reset_trig.iter())
            .zip(buf_reset_offset_01.iter())
        {
            if reset_trig || self.first_frame {
                self.first_frame = false;
                self.state_01 = reset_offset_01;
            } else {
                let state_delta = freq_hz / ctx.sample_rate_hz;
                self.state_01 += state_delta;
                self.state_01 = self.state_01 - (self.state_01 - 0.5).round();
            }
            let sample = self.waveform.sample(self.state_01, 0.0);
            self.buf.push(sample);
        }
    }

    // The pulse wave oscillator is specialized because in all other waveforms there's no need to
    // compute the values of the pulse width signal.
    fn sample_pulse(&mut self, ctx: &SigCtx) {
        let buf_freq_hz = self.freq_hz.sample(ctx);
        let buf_reset_trig = self.reset_trig.sample(ctx);
        let buf_reset_offset_01 = self.reset_offset_01.sample(ctx);
        let buf_pulse_width_01 = self.pulse_width_01.sample(ctx);
        self.buf.clear();
        for (((freq_hz, reset_trig), reset_offset_01), pulse_width_01) in
            buf_freq_hz
                .iter()
                .zip(buf_reset_trig.iter())
                .zip(buf_reset_offset_01.iter())
                .zip(buf_pulse_width_01.iter())
        {
            if reset_trig || self.first_frame {
                self.first_frame = false;
                self.state_01 = reset_offset_01;
            } else {
                let state_delta = freq_hz / ctx.sample_rate_hz;
                self.state_01 = (self.state_01 + state_delta).rem_euclid(1.0);
            }
            let sample = self.waveform.sample(self.state_01, pulse_width_01);
            self.buf.push(sample);
        }
    }
}

builder! {
    #[constructor = "oscillator"]
    #[constructor_doc = "A signal which oscillates with a given waveform at a given freq_hzuency."]
    #[build_fn = "Oscillator::new"]
    #[build_ty = "Sig<Oscillator<W, F, P, R, T>>"]
    #[generic_setter_type_name = "X"]
    pub struct OscillatorBuilder {
        #[generic_with_constraint = "Waveform"]
        #[generic_name = "W"]
        waveform: _,
        #[generic_with_constraint = "SigT<Item = f32>"]
        #[generic_name = "F"]
        freq_hz: _,
        #[generic_with_constraint = "SigT<Item = f32>"]
        #[default = 0.5]
        #[generic_name = "P"]
        pulse_width_01: f32,
        #[generic_with_constraint = "SigT<Item = f32>"]
        #[default = 0.0]
        #[generic_name = "R"]
        reset_offset_01: f32,
        #[generic_with_constraint = "SigT<Item = bool>"]
        #[default = false]
        #[generic_name = "T"]
        reset_trig: bool,
    }
}

pub struct OscillatorSaw<F, R, T>(Oscillator<Saw, F, f32, R, T>)
where
    F: SigT<Item = f32>,
    R: SigT<Item = f32>,
    T: SigT<Item = bool>;

impl<F, R, T> SigT for OscillatorSaw<F, R, T>
where
    F: SigT<Item = f32>,
    R: SigT<Item = f32>,
    T: SigT<Item = bool>,
{
    type Item = f32;

    fn sample(&mut self, ctx: &SigCtx) -> impl Buf<Self::Item> {
        self.0.sample_non_pulse(ctx);
        &self.0.buf
    }
}

impl<F, R, T> OscillatorSaw<F, R, T>
where
    F: SigT<Item = f32>,
    R: SigT<Item = f32>,
    T: SigT<Item = bool>,
{
    fn new(freq_hz: F, reset_offset_01: R, reset_trig: T) -> Sig<Self> {
        Sig(Self(
            Oscillator::new(Saw, freq_hz, 0., reset_offset_01, reset_trig).0,
        ))
    }
}

builder! {
    #[constructor = "saw"]
    #[constructor_doc = "A signal which oscillates with a saw wave at a given freq_hzuency."]
    #[build_fn = "OscillatorSaw::new"]
    #[build_ty = "Sig<OscillatorSaw<F, R, T>>"]
    #[generic_setter_type_name = "X"]
    pub struct OscillatorSawBuilder {
        #[generic_with_constraint = "SigT<Item = f32>"]
        #[generic_name = "F"]
        freq_hz: _,
        #[generic_with_constraint = "SigT<Item = f32>"]
        #[default = 0.0]
        #[generic_name = "R"]
        reset_offset_01: f32,
        #[generic_with_constraint = "SigT<Item = bool>"]
        #[default = false]
        #[generic_name = "T"]
        reset_trig: bool,
    }
}

pub struct OscillatorSine<F, R, T>(Oscillator<Sine, F, f32, R, T>)
where
    F: SigT<Item = f32>,
    R: SigT<Item = f32>,
    T: SigT<Item = bool>;

impl<F, R, T> SigT for OscillatorSine<F, R, T>
where
    F: SigT<Item = f32>,
    R: SigT<Item = f32>,
    T: SigT<Item = bool>,
{
    type Item = f32;

    fn sample(&mut self, ctx: &SigCtx) -> impl Buf<Self::Item> {
        self.0.sample_non_pulse(ctx);
        &self.0.buf
    }
}

impl<F, R, T> OscillatorSine<F, R, T>
where
    F: SigT<Item = f32>,
    R: SigT<Item = f32>,
    T: SigT<Item = bool>,
{
    fn new(freq_hz: F, reset_offset_01: R, reset_trig: T) -> Sig<Self> {
        Sig(Self(
            Oscillator::new(Sine, freq_hz, 0., reset_offset_01, reset_trig).0,
        ))
    }
}

builder! {
    #[constructor = "sine"]
    #[constructor_doc = "A signal which oscillates with a sine wave at a given freq_hzuency."]
    #[build_fn = "OscillatorSine::new"]
    #[build_ty = "Sig<OscillatorSine<F, R, T>>"]
    #[generic_setter_type_name = "X"]
    pub struct OscillatorSineBuilder {
        #[generic_with_constraint = "SigT<Item = f32>"]
        #[generic_name = "F"]
        freq_hz: _,
        #[generic_with_constraint = "SigT<Item = f32>"]
        #[default = 0.0]
        #[generic_name = "R"]
        reset_offset_01: f32,
        #[generic_with_constraint = "SigT<Item = bool>"]
        #[default = false]
        #[generic_name = "T"]
        reset_trig: bool,
    }
}

pub struct OscillatorTriangle<F, R, T>(Oscillator<Triangle, F, f32, R, T>)
where
    F: SigT<Item = f32>,
    R: SigT<Item = f32>,
    T: SigT<Item = bool>;

impl<F, R, T> SigT for OscillatorTriangle<F, R, T>
where
    F: SigT<Item = f32>,
    R: SigT<Item = f32>,
    T: SigT<Item = bool>,
{
    type Item = f32;

    fn sample(&mut self, ctx: &SigCtx) -> impl Buf<Self::Item> {
        self.0.sample_non_pulse(ctx);
        &self.0.buf
    }
}

impl<F, R, T> OscillatorTriangle<F, R, T>
where
    F: SigT<Item = f32>,
    R: SigT<Item = f32>,
    T: SigT<Item = bool>,
{
    fn new(freq_hz: F, reset_offset_01: R, reset_trig: T) -> Sig<Self> {
        Sig(Self(
            Oscillator::new(Triangle, freq_hz, 0., reset_offset_01, reset_trig)
                .0,
        ))
    }
}

builder! {
    #[constructor = "triangle"]
    #[constructor_doc = "A signal which oscillates with a triangle wave at a given freq_hzuency."]
    #[build_fn = "OscillatorTriangle::new"]
    #[build_ty = "Sig<OscillatorTriangle<F, R, T>>"]
    #[generic_setter_type_name = "X"]
    pub struct OscillatorTriangleBuilder {
        #[generic_with_constraint = "SigT<Item = f32>"]
        #[generic_name = "F"]
        freq_hz: _,
        #[generic_with_constraint = "SigT<Item = f32>"]
        #[default = 0.0]
        #[generic_name = "R"]
        reset_offset_01: f32,
        #[generic_with_constraint = "SigT<Item = bool>"]
        #[default = false]
        #[generic_name = "T"]
        reset_trig: bool,
    }
}

pub struct OscillatorPulse<F, P, R, T>(Oscillator<Pulse, F, P, R, T>)
where
    F: SigT<Item = f32>,
    P: SigT<Item = f32>,
    R: SigT<Item = f32>,
    T: SigT<Item = bool>;

impl<F, P, R, T> SigT for OscillatorPulse<F, P, R, T>
where
    F: SigT<Item = f32>,
    P: SigT<Item = f32>,
    R: SigT<Item = f32>,
    T: SigT<Item = bool>,
{
    type Item = f32;

    fn sample(&mut self, ctx: &SigCtx) -> impl Buf<Self::Item> {
        self.0.sample_pulse(ctx);
        &self.0.buf
    }
}

impl<F, P, R, T> OscillatorPulse<F, P, R, T>
where
    F: SigT<Item = f32>,
    P: SigT<Item = f32>,
    R: SigT<Item = f32>,
    T: SigT<Item = bool>,
{
    fn new(
        freq_hz: F,
        pulse_width_01: P,
        reset_offset_01: R,
        reset_trig: T,
    ) -> Sig<Self> {
        Sig(Self(
            Oscillator::new(
                Pulse,
                freq_hz,
                pulse_width_01,
                reset_offset_01,
                reset_trig,
            )
            .0,
        ))
    }
}

builder! {
    #[constructor = "pulse"]
    #[constructor_doc = "A signal which oscillates with a pulse wave at a given freq_hzuency."]
    #[build_fn = "OscillatorPulse::new"]
    #[build_ty = "Sig<OscillatorPulse<F, P, R, T>>"]
    #[generic_setter_type_name = "X"]
    pub struct OscillatorPulseBuilder {
        #[generic_with_constraint = "SigT<Item = f32>"]
        #[generic_name = "F"]
        freq_hz: _,
        #[generic_with_constraint = "SigT<Item = f32>"]
        #[default = 0.5]
        #[generic_name = "P"]
        pulse_width_01: f32,
        #[generic_with_constraint = "SigT<Item = f32>"]
        #[default = 0.0]
        #[generic_name = "R"]
        reset_offset_01: f32,
        #[generic_with_constraint = "SigT<Item = bool>"]
        #[default = false]
        #[generic_name = "T"]
        reset_trig: bool,
    }
}
