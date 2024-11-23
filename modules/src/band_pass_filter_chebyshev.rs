use crate::low_level::biquad_band_pass_filter::chebyshev;
use caw_builder_proc_macros::builder;
use caw_core_next::{Buf, Filter, SigCtx, SigT};
use itertools::izip;

pub struct Props<L, U, R>
where
    L: SigT<Item = f32>,
    U: SigT<Item = f32>,
    R: SigT<Item = f32>,
{
    lower_cutoff_hz: L,
    upper_cutoff_hz: U,
    resonance: R,
    filter_order_half: usize,
}

impl<L, U, R> Props<L, U, R>
where
    L: SigT<Item = f32>,
    U: SigT<Item = f32>,
    R: SigT<Item = f32>,
{
    fn new(
        lower_cutoff_hz: L,
        upper_cutoff_hz: U,
        resonance: R,
        filter_order_half: usize,
    ) -> Self {
        Self {
            lower_cutoff_hz,
            upper_cutoff_hz,
            resonance,
            filter_order_half,
        }
    }
}

builder! {
    #[constructor = "band_pass_filter_chebyshev"]
    #[constructor_doc = "A band pass filter with adjustable resonance"]
    #[build_fn = "Props::new"]
    #[build_ty = "Props<L, U, R>"]
    #[generic_setter_type_name = "X"]
    pub struct PropsBuilder {
        #[generic_with_constraint = "SigT<Item = f32>"]
        #[generic_name = "L"]
        lower_cutoff_hz: _,
        #[generic_with_constraint = "SigT<Item = f32>"]
        #[generic_name = "U"]
        upper_cutoff_hz: _,
        #[generic_with_constraint = "SigT<Item = f32>"]
        #[generic_name = "R"]
        #[default = 0.0]
        resonance: f32,
        #[default = 1]
        filter_order_half: usize,
    }
}

impl<L, U, R> Filter for PropsBuilder<L, U, R>
where
    L: SigT<Item = f32>,
    U: SigT<Item = f32>,
    R: SigT<Item = f32>,
{
    type ItemIn = f32;

    type Out<S> = BandPassFilterChebyshev<S, L, U, R>
    where
        S: SigT<Item = Self::ItemIn>;

    fn into_sig<S>(self, sig: S) -> Self::Out<S>
    where
        S: SigT<Item = Self::ItemIn>,
    {
        let props = self.build();
        BandPassFilterChebyshev {
            state: chebyshev::State::new(props.filter_order_half),
            props,
            sig,
            buf: Vec::new(),
        }
    }
}

pub struct BandPassFilterChebyshev<S, L, U, R>
where
    L: SigT<Item = f32>,
    U: SigT<Item = f32>,
    R: SigT<Item = f32>,
{
    props: Props<L, U, R>,
    sig: S,
    state: chebyshev::State,
    buf: Vec<f32>,
}

impl<S, L, U, R> SigT for BandPassFilterChebyshev<S, L, U, R>
where
    S: SigT<Item = f32>,
    L: SigT<Item = f32>,
    U: SigT<Item = f32>,
    R: SigT<Item = f32>,
{
    type Item = f32;

    fn sample(&mut self, ctx: &SigCtx) -> impl Buf<Self::Item> {
        self.buf.resize(ctx.num_samples, 0.0);
        for (out, &sample, &lower_cutoff_hz, &upper_cutoff_hz, &resonance) in izip! {
            self.buf.iter_mut(),
            self.sig.sample(ctx).iter(),
            self.props.lower_cutoff_hz.sample(ctx).iter(),
            self.props.upper_cutoff_hz.sample(ctx).iter(),
            self.props.resonance.sample(ctx).iter(),
        } {
            *out = self.state.run(
                sample,
                ctx.sample_rate_hz,
                lower_cutoff_hz,
                upper_cutoff_hz,
                resonance,
            );
        }
        &self.buf
    }
}

pub struct PropsCentered<C, W, M, R>
where
    C: SigT<Item = f32>,
    W: SigT<Item = f32>,
    R: SigT<Item = f32>,
{
    mid_cutoff_hz: C,
    width_cutoff_ratio: W,
    min_cutoff_hz: M,
    resonance: R,
    filter_order_half: usize,
}

impl<C, W, M, R> PropsCentered<C, W, M, R>
where
    C: SigT<Item = f32>,
    W: SigT<Item = f32>,
    M: SigT<Item = f32>,
    R: SigT<Item = f32>,
{
    fn new(
        mid_cutoff_hz: C,
        width_cutoff_ratio: W,
        min_cutoff_hz: M,
        resonance: R,
        filter_order_half: usize,
    ) -> Self {
        Self {
            mid_cutoff_hz,
            width_cutoff_ratio,
            min_cutoff_hz,
            resonance,
            filter_order_half,
        }
    }
}

builder! {
    #[constructor = "band_pass_filter_chebyshev_centered"]
    #[constructor_doc = "A band pass filter with adjustable resonance"]
    #[build_fn = "PropsCentered::new"]
    #[build_ty = "PropsCentered<C, W, M, R>"]
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
        #[generic_with_constraint = "SigT<Item = f32>"]
        #[generic_name = "R"]
        #[default = 0.0]
        resonance: f32,
        #[default = 1]
        filter_order_half: usize,
    }
}

impl<C, W, M, R> Filter for PropsCenteredBuilder<C, W, M, R>
where
    C: SigT<Item = f32>,
    W: SigT<Item = f32>,
    M: SigT<Item = f32>,
    R: SigT<Item = f32>,
{
    type ItemIn = f32;

    type Out<S> = BandPassFilterChebyshevCentered<S, C, W, M, R>
    where
        S: SigT<Item = Self::ItemIn>;

    fn into_sig<S>(self, sig: S) -> Self::Out<S>
    where
        S: SigT<Item = Self::ItemIn>,
    {
        let props = self.build();
        BandPassFilterChebyshevCentered {
            state: chebyshev::State::new(props.filter_order_half),
            props,
            sig,
            buf: Vec::new(),
        }
    }
}

pub struct BandPassFilterChebyshevCentered<S, C, W, M, R>
where
    C: SigT<Item = f32>,
    W: SigT<Item = f32>,
    M: SigT<Item = f32>,
    R: SigT<Item = f32>,
{
    props: PropsCentered<C, W, M, R>,
    sig: S,
    state: chebyshev::State,
    buf: Vec<f32>,
}

impl<S, C, W, M, R> SigT for BandPassFilterChebyshevCentered<S, C, W, M, R>
where
    S: SigT<Item = f32>,
    C: SigT<Item = f32>,
    W: SigT<Item = f32>,
    M: SigT<Item = f32>,
    R: SigT<Item = f32>,
{
    type Item = f32;

    fn sample(&mut self, ctx: &SigCtx) -> impl Buf<Self::Item> {
        self.buf.resize(ctx.num_samples, 0.0);
        for (
            out,
            &sample,
            &mid_cutoff_hz,
            &width_cutoff_ratio,
            &min_cutoff_hz,
            &resonance,
        ) in izip! {
            self.buf.iter_mut(),
            self.sig.sample(ctx).iter(),
            self.props.mid_cutoff_hz.sample(ctx).iter(),
            self.props.width_cutoff_ratio.sample(ctx).iter(),
            self.props.min_cutoff_hz.sample(ctx).iter(),
            self.props.resonance.sample(ctx).iter(),
        } {
            let width_cutoff_hz = width_cutoff_ratio * mid_cutoff_hz;
            let lower_cutoff_hz =
                (mid_cutoff_hz - (width_cutoff_hz / 2.0)).max(min_cutoff_hz);
            let upper_cutoff_hz = lower_cutoff_hz + width_cutoff_hz;
            *out = self.state.run(
                sample,
                ctx.sample_rate_hz,
                lower_cutoff_hz,
                upper_cutoff_hz,
                resonance,
            );
        }
        &self.buf
    }
}
