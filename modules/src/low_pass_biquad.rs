use crate::low_level::biquad_filter::BiquadFilterLowPass;
use caw_builder_proc_macros::builder;
use caw_core::{Buf, Filter, SigCtx, SigT};
use itertools::izip;

builder! {
    #[constructor = "low_pass_biquad"]
    #[constructor_doc = "A low pass biquad filter"]
    #[generic_setter_type_name = "X"]
    pub struct Props {
        #[generic_with_constraint = "SigT<Item = f32>"]
        #[generic_name = "C"]
        cutoff_hz: _,
        #[generic_with_constraint = "SigT<Item = f32>"]
        #[generic_name = "R"]
        #[default = 0.]
        resonance: f32,
    }
}

impl<C: SigT<Item = f32>, R: SigT<Item = f32>> Props<C, R> {
    pub fn q<X>(self, resonance: X) -> Props<C, X>
    where
        X: SigT<Item = f32>,
    {
        self.resonance(resonance)
    }
}

impl<C, R> Filter for Props<C, R>
where
    C: SigT<Item = f32>,
    R: SigT<Item = f32>,
{
    type ItemIn = f32;

    type Out<S>
        = LowPassBiquad<S, C, R>
    where
        S: SigT<Item = Self::ItemIn>;

    fn into_sig<S>(self, sig: S) -> Self::Out<S>
    where
        S: SigT<Item = Self::ItemIn>,
    {
        LowPassBiquad {
            props: self,
            sig,
            low_level: BiquadFilterLowPass::new(),
            buf: Vec::new(),
        }
    }
}

pub struct LowPassBiquad<S, C, R>
where
    S: SigT<Item = f32>,
    C: SigT<Item = f32>,
    R: SigT<Item = f32>,
{
    props: Props<C, R>,
    sig: S,
    low_level: BiquadFilterLowPass,
    buf: Vec<f32>,
}

impl<S, C, R> SigT for LowPassBiquad<S, C, R>
where
    S: SigT<Item = f32>,
    C: SigT<Item = f32>,
    R: SigT<Item = f32>,
{
    type Item = f32;

    fn sample(&mut self, ctx: &SigCtx) -> impl Buf<Self::Item> {
        self.buf.resize(ctx.num_samples, 0.0);
        let sig = self.sig.sample(ctx);
        let cutoff_hz = self.props.cutoff_hz.sample(ctx);
        let resonance = self.props.resonance.sample(ctx);
        for (out, sample, cutoff_hz, resonance) in izip! {
            self.buf.iter_mut(),
            sig.iter(),
            cutoff_hz.iter(),
            resonance.iter(),
        } {
            *out = self.low_level.process(
                sample as f64,
                cutoff_hz as f64,
                resonance as f64,
                ctx.sample_rate_hz as f64,
            ) as f32;
        }
        &self.buf
    }
}
