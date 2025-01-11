use caw_builder_proc_macros::builder;
use caw_core::{Buf, Filter, SigCtx, SigT};
use itertools::izip;

builder! {
    #[constructor = "quantizer"]
    #[constructor_doc = "Quantize a signal with a configurable resolution"]
    #[generic_setter_type_name = "X"]
    pub struct Props {
        // If the input signal exceeds the threshold then the excess will be multiplied by the ratio.
        #[generic_with_constraint = "SigT<Item = f32>"]
        #[generic_name = "R"]
        resolution: _,
    }
}

impl<R> Filter for Props<R>
where
    R: SigT<Item = f32>,
{
    type ItemIn = f32;

    type Out<S>
        = Quantizer<R, S>
    where
        S: SigT<Item = Self::ItemIn>;

    fn into_sig<S>(self, sig: S) -> Self::Out<S>
    where
        S: SigT<Item = Self::ItemIn>,
    {
        Quantizer {
            props: self,
            sig,
            buf: Vec::new(),
        }
    }
}

pub struct Quantizer<R, S>
where
    R: SigT<Item = f32>,
    S: SigT<Item = f32>,
{
    props: Props<R>,
    sig: S,
    buf: Vec<f32>,
}

impl<R, S> SigT for Quantizer<R, S>
where
    R: SigT<Item = f32>,
    S: SigT<Item = f32>,
{
    type Item = f32;

    fn sample(&mut self, ctx: &SigCtx) -> impl Buf<Self::Item> {
        self.buf.resize(ctx.num_samples, 0.0);
        let sig = self.sig.sample(ctx);
        let resolution = self.props.resolution.sample(ctx);
        for (out, sample, resolution) in izip! {
            self.buf.iter_mut(),
            sig.iter(),
            resolution.iter(),
        } {
            let resolution = resolution.max(1.0);
            *out = (sample * resolution).floor() / resolution;
        }
        &self.buf
    }
}
