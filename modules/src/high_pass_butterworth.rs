use crate::low_level::biquad_filter::butterworth;
use caw_builder_proc_macros::builder;
use caw_core_next::{Buf, Filter, SigCtx, SigT};
use itertools::izip;

pub struct Props<C>
where
    C: SigT<Item = f32>,
{
    cutoff_hz: C,
    filter_order_half: usize,
}

impl<C> Props<C>
where
    C: SigT<Item = f32>,
{
    fn new(cutoff_hz: C, filter_order_half: usize) -> Self {
        Self {
            cutoff_hz,
            filter_order_half,
        }
    }
}

builder! {
    #[constructor = "high_pass_butterworth"]
    #[constructor_doc = "A basic high pass filter"]
    #[build_fn = "Props::new"]
    #[build_ty = "Props<C>"]
    #[generic_setter_type_name = "X"]
    pub struct PropsBuilder {
        #[generic_with_constraint = "SigT<Item = f32>"]
        #[generic_name = "C"]
        cutoff_hz: _,
        #[default = 1]
        filter_order_half: usize,
    }
}

impl<C> Filter for PropsBuilder<C>
where
    C: SigT<Item = f32>,
{
    type ItemIn = f32;

    type Out<S> = HighPassButterworth<S, C>
    where
        S: SigT<Item = Self::ItemIn>;

    fn into_sig<S>(self, sig: S) -> Self::Out<S>
    where
        S: SigT<Item = Self::ItemIn>,
    {
        let props = self.build();
        HighPassButterworth {
            state: butterworth::State::new(props.filter_order_half),
            props,
            sig,
            buf: Vec::new(),
        }
    }
}

pub struct HighPassButterworth<S, C>
where
    S: SigT<Item = f32>,
    C: SigT<Item = f32>,
{
    props: Props<C>,
    sig: S,
    state: butterworth::State,
    buf: Vec<f32>,
}

impl<S, C> SigT for HighPassButterworth<S, C>
where
    S: SigT<Item = f32>,
    C: SigT<Item = f32>,
{
    type Item = f32;

    fn sample(&mut self, ctx: &SigCtx) -> impl Buf<Self::Item> {
        self.buf.resize(ctx.num_samples, 0.0);
        for (out, &sample, &cutoff_hz) in izip! {
            self.buf.iter_mut(),
            self.sig.sample(ctx).iter(),
            self.props.cutoff_hz.sample(ctx).iter(),
        } {
            *out = butterworth::high_pass::run(
                &mut self.state,
                sample as f64,
                ctx.sample_rate_hz as f64,
                cutoff_hz as f64,
            ) as f32;
        }
        &self.buf
    }
}
