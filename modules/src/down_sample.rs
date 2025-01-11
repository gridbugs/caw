use caw_builder_proc_macros::builder;
use caw_core::{Buf, Filter, SigCtx, SigT};
use itertools::izip;

builder! {
    #[constructor = "down_sample"]
    #[constructor_doc = "Artificially reduce the sample rate of a signal"]
    #[generic_setter_type_name = "X"]
    pub struct Props {
        // If the input signal exceeds the threshold then the excess will be multiplied by the ratio.
        #[generic_with_constraint = "SigT<Item = f32>"]
        #[generic_name = "C"]
        scale: _,
    }
}

impl<C> Filter for Props<C>
where
    C: SigT<Item = f32>,
{
    type ItemIn = f32;

    type Out<S>
        = Downsample<C, S>
    where
        S: SigT<Item = Self::ItemIn>;

    fn into_sig<S>(self, sig: S) -> Self::Out<S>
    where
        S: SigT<Item = Self::ItemIn>,
    {
        Downsample {
            props: self,
            sig,
            buf: Vec::new(),
            prev_input_sample: 0.0,
            prev_output_sample: 0.0,
            remaining_samples: 0.0,
        }
    }
}

pub struct Downsample<C, S>
where
    C: SigT<Item = f32>,
    S: SigT<Item = f32>,
{
    props: Props<C>,
    sig: S,
    buf: Vec<f32>,
    prev_input_sample: f32,
    prev_output_sample: f32,
    remaining_samples: f32,
}

impl<C, S> SigT for Downsample<C, S>
where
    C: SigT<Item = f32>,
    S: SigT<Item = f32>,
{
    type Item = f32;

    fn sample(&mut self, ctx: &SigCtx) -> impl Buf<Self::Item> {
        self.buf.resize(ctx.num_samples, 0.0);
        let sig = self.sig.sample(ctx);
        let scale = self.props.scale.sample(ctx);
        for (out, sample, scale) in izip! {
            self.buf.iter_mut(),
            sig.iter(),
            scale.iter(),
        } {
            let scale = scale.max(1.0);
            if self.remaining_samples < 1.0 {
                self.remaining_samples = self.remaining_samples.max(0.0);
                self.prev_output_sample = (self.prev_input_sample
                    * self.remaining_samples)
                    + (sample * (1.0 - self.remaining_samples));
                // linearly interpolate between the current and previous samples
                *out = self.prev_output_sample;
                self.remaining_samples = scale;
            } else {
                *out = self.prev_output_sample;
                self.remaining_samples -= 1.0;
            }
            self.prev_input_sample = sample;
        }
        &self.buf
    }
}
