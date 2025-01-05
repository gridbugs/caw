use caw_builder_proc_macros::builder;
use caw_core_next::{Buf, Filter, SigCtx, SigT};
use itertools::izip;

builder! {
    #[constructor = "compressor"]
    #[constructor_doc = "Soft clamps a signal within a configurable threshold"]
    #[generic_setter_type_name = "X"]
    pub struct Props {
        // If the input signal exceeds the threshold then the excess will be multiplied by the ratio.
        #[generic_with_constraint = "SigT<Item = f32>"]
        #[generic_name = "T"]
        #[default = 1.0]
        threshold: f32,
        #[generic_with_constraint = "SigT<Item = f32>"]
        #[generic_name = "R"]
        #[default = 0.0]
        ratio: f32,
        // Pre-amplify the input by this amount.
        #[generic_with_constraint = "SigT<Item = f32>"]
        #[generic_name = "S"]
        #[default = 1.0]
        scale: f32,
    }
}

impl<T, R, S> Filter for Props<T, R, S>
where
    T: SigT<Item = f32>,
    R: SigT<Item = f32>,
    S: SigT<Item = f32>,
{
    type ItemIn = f32;

    type Out<I>
        = Compressor<I, T, R, S>
    where
        I: SigT<Item = Self::ItemIn>;

    fn into_sig<I>(self, sig: I) -> Self::Out<I>
    where
        I: SigT<Item = Self::ItemIn>,
    {
        Compressor {
            props: self,
            sig,
            buf: Vec::new(),
        }
    }
}

pub struct Compressor<I, T, R, S>
where
    I: SigT<Item = f32>,
    T: SigT<Item = f32>,
    R: SigT<Item = f32>,
    S: SigT<Item = f32>,
{
    props: Props<T, R, S>,
    sig: I,
    buf: Vec<f32>,
}

impl<I, T, R, S> SigT for Compressor<I, T, R, S>
where
    I: SigT<Item = f32>,
    T: SigT<Item = f32>,
    R: SigT<Item = f32>,
    S: SigT<Item = f32>,
{
    type Item = f32;

    fn sample(&mut self, ctx: &SigCtx) -> impl Buf<Self::Item> {
        self.buf.resize(ctx.num_samples, 0.0);
        for (out, sample, threshold, ratio, scale) in izip! {
            self.buf.iter_mut(),
            self.sig.sample(ctx).iter(),
            self.props.threshold.sample(ctx).iter(),
            self.props.ratio.sample(ctx).iter(),
            self.props.scale.sample(ctx).iter(),
        } {
            let sample = sample * scale;
            let sample_abs = sample.abs();
            *out = if sample_abs > threshold {
                let delta = sample_abs - threshold;
                let delta_scaled = delta * ratio;
                (threshold + delta_scaled) * sample.signum()
            } else {
                sample
            };
        }
        &self.buf
    }
}
