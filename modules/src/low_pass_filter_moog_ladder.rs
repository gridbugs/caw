use caw_builder_proc_macros::builder;
use caw_core_next::{Buf, Flt, Sig, SigCtx, SigT};

pub struct LowPassFilterMoogLadderFlt<C, R>
where
    C: SigT<Item = f32>,
    R: SigT<Item = f32>,
{
    cutoff_hz: C,
    resonance: R,
}

impl<C, R> LowPassFilterMoogLadderFlt<C, R>
where
    C: SigT<Item = f32>,
    R: SigT<Item = f32>,
{
    fn new(cutoff_hz: C, resonance: R) -> Self {
        Self {
            cutoff_hz,
            resonance,
        }
    }
}

builder! {
    #[constructor = "low_pass_filter_moog_ladder"]
    #[constructor_doc = "A low pass filter with adjustable resonance"]
    #[build_fn = "LowPassFilterMoogLadderFlt::new"]
    #[build_ty = "LowPassFilterMoogLadderFlt<C, R>"]
    #[generic_setter_type_name = "X"]
    pub struct LowPassFilterMoogLadderFltBuilder {
        #[generic_with_constraint = "SigT<Item = f32>"]
        #[generic_name = "C"]
        cutoff_hz: _,
        #[generic_with_constraint = "SigT<Item = f32>"]
        #[generic_name = "R"]
        #[default = 0.0]
        resonance: f32,
    }
}

impl<C, R> Flt for LowPassFilterMoogLadderFlt<C, R>
where
    C: SigT<Item = f32>,
    R: SigT<Item = f32>,
{
    type Out<S> = LowPassFilterMoogLadderSig<S, C, R>
    where
        S: SigT<Item = f32>;

    fn into_sig<S>(self, sig: S) -> Self::Out<S>
    where
        S: SigT<Item = f32>,
    {
        LowPassFilterMoogLadderSig {
            cutoff_hz: self.cutoff_hz,
            resonance: self.resonance,
            sig,
            buf: Vec::new(),
        }
    }
}

pub struct LowPassFilterMoogLadderSig<S, C, R>
where
    S: SigT<Item = f32>,
    C: SigT<Item = f32>,
    R: SigT<Item = f32>,
{
    cutoff_hz: C,
    resonance: R,
    sig: S,
    buf: Vec<f32>,
}

impl<S, C, R> SigT for LowPassFilterMoogLadderSig<S, C, R>
where
    S: SigT<Item = f32>,
    C: SigT<Item = f32>,
    R: SigT<Item = f32>,
{
    type Item = f32;

    fn sample(&mut self, ctx: &SigCtx) -> impl Buf<Self::Item> {
        &self.buf
    }
}
