use crate::low_level::biquad_filter::chebyshev;
use caw_builder_proc_macros::builder;
use caw_core_next::{Buf, Filter, SigCtx, SigT};
use itertools::izip;

pub struct Props<C, R>
where
    C: SigT<Item = f32>,
    R: SigT<Item = f32>,
{
    cutoff_hz: C,
    resonance: R,
    filter_order_half: usize,
}

impl<C, R> Props<C, R>
where
    C: SigT<Item = f32>,
    R: SigT<Item = f32>,
{
    fn new(cutoff_hz: C, resonance: R, filter_order_half: usize) -> Self {
        Self {
            cutoff_hz,
            resonance,
            filter_order_half,
        }
    }
}

builder! {
    #[constructor = "low_pass_filter_chebyshev"]
    #[constructor_doc = "A low pass filter with adjustable resonance"]
    #[build_fn = "Props::new"]
    #[build_ty = "Props<C, R>"]
    #[generic_setter_type_name = "X"]
    pub struct PropsBuilder {
        #[generic_with_constraint = "SigT<Item = f32>"]
        #[generic_name = "C"]
        cutoff_hz: _,
        #[generic_with_constraint = "SigT<Item = f32>"]
        #[generic_name = "R"]
        #[default = 0.0]
        resonance: f32,
        #[default = 1]
        filter_order_half: usize,
    }
}

impl<C, R> Filter for PropsBuilder<C, R>
where
    C: SigT<Item = f32>,
    R: SigT<Item = f32>,
{
    type ItemIn = f32;

    type Out<S> = LowPassFilterChebyshev<S, C, R>
    where
        S: SigT<Item = Self::ItemIn>;

    fn into_sig<S>(self, sig: S) -> Self::Out<S>
    where
        S: SigT<Item = Self::ItemIn>,
    {
        let props = self.build();
        LowPassFilterChebyshev {
            state: chebyshev::State::new(props.filter_order_half),
            props,
            sig,
            buf: Vec::new(),
        }
    }
}

pub struct LowPassFilterChebyshev<S, C, R>
where
    S: SigT<Item = f32>,
    C: SigT<Item = f32>,
    R: SigT<Item = f32>,
{
    props: Props<C, R>,
    sig: S,
    state: chebyshev::State,
    buf: Vec<f32>,
}

impl<S, C, R> SigT for LowPassFilterChebyshev<S, C, R>
where
    S: SigT<Item = f32>,
    C: SigT<Item = f32>,
    R: SigT<Item = f32>,
{
    type Item = f32;

    fn sample(&mut self, ctx: &SigCtx) -> impl Buf<Self::Item> {
        self.buf.resize(ctx.num_samples, 0.0);
        for (out, &sample, &cutoff_hz, &resonance) in izip! {
            self.buf.iter_mut(),
            self.sig.sample(ctx).iter(),
            self.props.cutoff_hz.sample(ctx).iter(),
            self.props.resonance.sample(ctx).iter(),
        } {
            *out = chebyshev::low_pass::run(
                &mut self.state,
                sample,
                ctx.sample_rate_hz,
                cutoff_hz,
                resonance,
            );
        }
        &self.buf
    }
}
