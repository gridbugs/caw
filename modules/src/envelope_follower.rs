use crate::low_pass_butterworth::{self, LowPassButterworth};
use caw_builder_proc_macros::builder;
use caw_core_next::{Buf, Filter, Sig, SigAbs, SigCtx, SigT};

pub struct Props<S>
where
    S: SigT<Item = f32>,
{
    sensitivity_hz: S,
}

impl<S> Props<S>
where
    S: SigT<Item = f32>,
{
    fn new(sensitivity_hz: S) -> Self {
        Self { sensitivity_hz }
    }
}

pub const DEFAULT_SENSITIVITY_HZ: f32 = 60.0;

builder! {
    #[constructor = "envelope_follower"]
    #[constructor_doc = "Approximates the loudness of its input signal"]
    #[build_fn = "Props::new"]
    #[build_ty = "Props<S>"]
    #[generic_setter_type_name = "X"]
    pub struct PropsBuilder {
        #[generic_with_constraint = "SigT<Item = f32>"]
        #[generic_name = "S"]
        #[default = DEFAULT_SENSITIVITY_HZ]
        sensitivity_hz: f32,
    }
}

impl<S> Filter for PropsBuilder<S>
where
    S: SigT<Item = f32>,
{
    type ItemIn = f32;

    type Out<I>
        = EnvelopeFollower<I, S>
    where
        I: SigT<Item = Self::ItemIn>;

    fn into_sig<I>(self, sig: I) -> Self::Out<I>
    where
        I: SigT<Item = Self::ItemIn>,
    {
        let Props { sensitivity_hz } = self.build();
        let low_pass_filter =
            low_pass_butterworth::low_pass_butterworth(sensitivity_hz)
                .into_sig(Sig(sig).abs().0);
        EnvelopeFollower { low_pass_filter }
    }
}

pub struct EnvelopeFollower<I, S>
where
    I: SigT<Item = f32>,
    S: SigT<Item = f32>,
{
    // apply a low pass filter to the absolute value of samples from the input
    low_pass_filter: LowPassButterworth<SigAbs<I>, S>,
}

impl<I, S> SigT for EnvelopeFollower<I, S>
where
    I: SigT<Item = f32>,
    S: SigT<Item = f32>,
{
    type Item = f32;

    fn sample(&mut self, ctx: &SigCtx) -> impl Buf<Self::Item> {
        self.low_pass_filter.sample(ctx)
    }
}
