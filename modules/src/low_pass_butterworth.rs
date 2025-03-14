use crate::low_level::biquad_filter::butterworth;
use caw_builder_proc_macros::builder;
use caw_core::{Buf, Filter, SigCtx, SigT};
use itertools::izip;

builder! {
    #[constructor = "low_pass_butterworth"]
    #[constructor_doc = "A basic low pass filter"]
    #[generic_setter_type_name = "X"]
    pub struct Props {
        #[generic_with_constraint = "SigT<Item = f32>"]
        #[generic_name = "C"]
        cutoff_hz: _,
        #[default = 1]
        filter_order_half: usize,
    }
}

impl<C> Filter for Props<C>
where
    C: SigT<Item = f32>,
{
    type ItemIn = f32;

    type Out<S>
        = LowPassButterworth<S, C>
    where
        S: SigT<Item = Self::ItemIn>;

    fn into_sig<S>(self, sig: S) -> Self::Out<S>
    where
        S: SigT<Item = Self::ItemIn>,
    {
        LowPassButterworth {
            state: butterworth::State::new(self.filter_order_half),
            props: self,
            sig,
            buf: Vec::new(),
        }
    }
}

pub struct LowPassButterworth<S, C>
where
    S: SigT<Item = f32>,
    C: SigT<Item = f32>,
{
    props: Props<C>,
    sig: S,
    state: butterworth::State,
    buf: Vec<f32>,
}

impl<S, C> SigT for LowPassButterworth<S, C>
where
    S: SigT<Item = f32>,
    C: SigT<Item = f32>,
{
    type Item = f32;

    fn sample(&mut self, ctx: &SigCtx) -> impl Buf<Self::Item> {
        self.buf.resize(ctx.num_samples, 0.0);
        let sig = self.sig.sample(ctx);
        let cutoff_hz = self.props.cutoff_hz.sample(ctx);
        for (out, sample, cutoff_hz) in izip! {
            self.buf.iter_mut(),
            sig.iter(),
            cutoff_hz.iter(),
        } {
            *out = butterworth::low_pass::run(
                &mut self.state,
                sample as f64,
                ctx.sample_rate_hz as f64,
                cutoff_hz as f64,
            ) as f32;
        }
        &self.buf
    }
}
