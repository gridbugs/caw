use crate::low_level::biquad_band_pass_filter::butterworth;
use caw_builder_proc_macros::builder;
use caw_core::{Buf, Filter, SigCtx, SigT};
use itertools::izip;

builder! {
    #[constructor = "band_pass_butterworth"]
    #[constructor_doc = "A basic band pass filter"]
    #[generic_setter_type_name = "X"]
    pub struct Props {
        #[generic_with_constraint = "SigT<Item = f32>"]
        #[generic_name = "L"]
        lower_cutoff_hz: _,
        #[generic_with_constraint = "SigT<Item = f32>"]
        #[generic_name = "U"]
        upper_cutoff_hz: _,
        #[default = 1]
        filter_order_half: usize,
    }
}

impl<L, U> Filter for Props<L, U>
where
    L: SigT<Item = f32>,
    U: SigT<Item = f32>,
{
    type ItemIn = f32;

    type Out<S>
        = BandPassButterworth<S, L, U>
    where
        S: SigT<Item = Self::ItemIn>;

    fn into_sig<S>(self, sig: S) -> Self::Out<S>
    where
        S: SigT<Item = Self::ItemIn>,
    {
        BandPassButterworth {
            state: butterworth::State::new(self.filter_order_half),
            props: self,
            sig,
            buf: Vec::new(),
        }
    }
}

pub struct BandPassButterworth<S, L, U>
where
    L: SigT<Item = f32>,
    U: SigT<Item = f32>,
{
    props: Props<L, U>,
    sig: S,
    state: butterworth::State,
    buf: Vec<f32>,
}

impl<S, L, U> SigT for BandPassButterworth<S, L, U>
where
    S: SigT<Item = f32>,
    L: SigT<Item = f32>,
    U: SigT<Item = f32>,
{
    type Item = f32;

    fn sample(&mut self, ctx: &SigCtx) -> impl Buf<Self::Item> {
        self.buf.resize(ctx.num_samples, 0.0);
        for (out, sample, lower_cutoff_hz, upper_cutoff_hz) in izip! {
            self.buf.iter_mut(),
            self.sig.sample(ctx).iter(),
            self.props.lower_cutoff_hz.sample(ctx).iter(),
            self.props.upper_cutoff_hz.sample(ctx).iter(),
        } {
            *out = self.state.run(
                sample as f64,
                ctx.sample_rate_hz as f64,
                lower_cutoff_hz as f64,
                upper_cutoff_hz as f64,
            ) as f32;
        }
        &self.buf
    }
}

pub struct PropsCentered<C, W, M>
where
    C: SigT<Item = f32>,
    W: SigT<Item = f32>,
{
    mid_cutoff_hz: C,
    width_cutoff_ratio: W,
    min_cutoff_hz: M,
    filter_order_half: usize,
}

impl<C, W, M> PropsCentered<C, W, M>
where
    C: SigT<Item = f32>,
    W: SigT<Item = f32>,
    M: SigT<Item = f32>,
{
    fn new(
        mid_cutoff_hz: C,
        width_cutoff_ratio: W,
        min_cutoff_hz: M,
        filter_order_half: usize,
    ) -> Self {
        Self {
            mid_cutoff_hz,
            width_cutoff_ratio,
            min_cutoff_hz,
            filter_order_half,
        }
    }
}

builder! {
    #[constructor = "band_pass_butterworth_centered"]
    #[constructor_doc = "A basic band pass filter"]
    #[build_fn = "PropsCentered::new"]
    #[build_ty = "PropsCentered<C, W, M>"]
    #[generic_setter_type_name = "X"]
    pub struct PropsCenteredBuilder {
        #[generic_with_constraint = "SigT<Item = f32>"]
        #[generic_name = "C"]
        mid_cutoff_hz: _,
        #[generic_with_constraint = "SigT<Item = f32>"]
        #[generic_name = "W"]
        width_cutoff_ratio: _,
        #[generic_with_constraint = "SigT<Item = f32>"]
        #[generic_name = "M"]
        #[default = 20.0]
        min_cutoff_hz: f32,
        #[default = 1]
        filter_order_half: usize,
    }
}

impl<C, W, M> Filter for PropsCenteredBuilder<C, W, M>
where
    C: SigT<Item = f32>,
    W: SigT<Item = f32>,
    M: SigT<Item = f32>,
{
    type ItemIn = f32;

    type Out<S>
        = BandPassButterworthCentered<S, C, W, M>
    where
        S: SigT<Item = Self::ItemIn>;

    fn into_sig<S>(self, sig: S) -> Self::Out<S>
    where
        S: SigT<Item = Self::ItemIn>,
    {
        let props = self.build();
        BandPassButterworthCentered {
            state: butterworth::State::new(props.filter_order_half),
            props,
            sig,
            buf: Vec::new(),
        }
    }
}

pub struct BandPassButterworthCentered<S, C, W, M>
where
    C: SigT<Item = f32>,
    W: SigT<Item = f32>,
    M: SigT<Item = f32>,
{
    props: PropsCentered<C, W, M>,
    sig: S,
    state: butterworth::State,
    buf: Vec<f32>,
}

impl<S, C, W, M> SigT for BandPassButterworthCentered<S, C, W, M>
where
    S: SigT<Item = f32>,
    C: SigT<Item = f32>,
    W: SigT<Item = f32>,
    M: SigT<Item = f32>,
{
    type Item = f32;

    fn sample(&mut self, ctx: &SigCtx) -> impl Buf<Self::Item> {
        self.buf.resize(ctx.num_samples, 0.0);
        for (out, sample, mid_cutoff_hz, width_cutoff_ratio, min_cutoff_hz) in izip! {
            self.buf.iter_mut(),
            self.sig.sample(ctx).iter(),
            self.props.mid_cutoff_hz.sample(ctx).iter(),
            self.props.width_cutoff_ratio.sample(ctx).iter(),
            self.props.min_cutoff_hz.sample(ctx).iter(),
        } {
            let width_cutoff_hz = width_cutoff_ratio * mid_cutoff_hz;
            let lower_cutoff_hz =
                (mid_cutoff_hz - (width_cutoff_hz / 2.0)).max(min_cutoff_hz);
            let upper_cutoff_hz = lower_cutoff_hz + width_cutoff_hz;
            *out = self.state.run(
                sample as f64,
                ctx.sample_rate_hz as f64,
                lower_cutoff_hz as f64,
                upper_cutoff_hz as f64,
            ) as f32;
        }
        &self.buf
    }
}
