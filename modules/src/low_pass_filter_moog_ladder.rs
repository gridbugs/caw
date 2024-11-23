use crate::low_level::moog_ladder::OberheimVariationMoogState;
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
}

impl<C, R> Props<C, R>
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
    }
}

impl<C, R> Filter for PropsBuilder<C, R>
where
    C: SigT<Item = f32>,
    R: SigT<Item = f32>,
{
    type ItemIn = f32;

    type Out<S> = LowPassFilterMoogLadder<S, C, R>
    where
        S: SigT<Item = Self::ItemIn>;

    fn into_sig<S>(self, sig: S) -> Self::Out<S>
    where
        S: SigT<Item = Self::ItemIn>,
    {
        let props = self.build();
        LowPassFilterMoogLadder {
            props,
            sig,
            state: OberheimVariationMoogState::new(),
            buf: Vec::new(),
        }
    }
}

pub struct LowPassFilterMoogLadder<S, C, R>
where
    S: SigT<Item = f32>,
    C: SigT<Item = f32>,
    R: SigT<Item = f32>,
{
    props: Props<C, R>,
    sig: S,
    state: OberheimVariationMoogState,
    buf: Vec<f32>,
}

impl<S, C, R> SigT for LowPassFilterMoogLadder<S, C, R>
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
            self.state
                .update_params(ctx.sample_rate_hz, cutoff_hz, resonance);
            *out = self.state.process_sample(sample);
        }
        &self.buf
    }
}
