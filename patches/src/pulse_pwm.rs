use caw_builder_proc_macros::builder;
use caw_core::{
    sig_ops::{sig_add, sig_mul},
    Sig, SigT,
};
use caw_modules::*;
use oscillator::Oscillator;

pub type PulsePwm<O, L, S, R> = Sig<
    Oscillator<
        Pulse,
        O,
        sig_add::OpSigScalar<
            sig_mul::OpSigSig<
                Oscillator<Triangle, L, f32, R, bool>,
                sig_mul::OpSigScalar<S, f32>,
            >,
            f32,
        >,
        f32,
        bool,
    >,
>;

fn make<O, L, S, R>(
    osc_freq_hz: O,
    lfo_freq_hz: L,
    lfo_scale_01: S,
    lfo_reset_offset_01: R,
) -> PulsePwm<O, L, S, R>
where
    O: SigT<Item = f32>,
    L: SigT<Item = f32>,
    S: SigT<Item = f32>,
    R: SigT<Item = f32>,
{
    let lfo = (oscillator(Triangle, lfo_freq_hz)
        .reset_offset_01(lfo_reset_offset_01)
        .build()
        * (Sig(lfo_scale_01) * 0.5))
        + 0.5;
    oscillator(Pulse, osc_freq_hz).pulse_width_01(lfo.0).build()
}

builder! {
    #[constructor = "pulse_pwm"]
    #[constructor_doc = "A pulse oscillator whose pwm varies with a built-in lfo"]
    #[build_fn = "make"]
    #[build_ty = "PulsePwm<O, L, S, R>"]
    #[generic_setter_type_name = "X"]
    pub struct Props {
        #[generic_with_constraint = "SigT<Item = f32>"]
        #[generic_name = "O"]
        osc_freq_hz: _,
        #[generic_with_constraint = "SigT<Item = f32>"]
        #[generic_name = "L"]
        #[default = 1.0]
        lfo_freq_hz: f32,
        #[generic_with_constraint = "SigT<Item = f32>"]
        #[generic_name = "S"]
        #[default = 0.5]
        lfo_scale_01: f32,
        #[generic_with_constraint = "SigT<Item = f32>"]
        #[generic_name = "R"]
        #[default = 0.0]
        lfo_reset_offset_01: f32,
    }
}
