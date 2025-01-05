use crate::low_level::linearly_interpolating_ring_buffer::LinearlyInterpolatingRingBuffer;
use caw_builder_proc_macros::builder;
use caw_core_next::{Buf, Filter, SigCtx, SigT};
use itertools::izip;

pub struct Props<P, M, F>
where
    P: SigT<Item = f32>,
    M: SigT<Item = f32>,
    F: SigT<Item = f32>,
{
    period_s: P,
    // 0 is dry signal, 1 is all delay
    mix_01: M,
    // ratio of output fed back into the input
    feedback_ratio: F,
}

impl<P, M, F> Props<P, M, F>
where
    P: SigT<Item = f32>,
    M: SigT<Item = f32>,
    F: SigT<Item = f32>,
{
    fn new(period_s: P, mix_01: M, feedback_ratio: F) -> Self {
        Self {
            period_s,
            mix_01,
            feedback_ratio,
        }
    }
}

builder! {
    #[constructor = "delay_periodic_s"]
    #[constructor_doc = "Delay module where the delay is a configurable period"]
    #[build_fn = "Props::new"]
    #[build_ty = "Props<P, M, F>"]
    #[generic_setter_type_name = "X"]
    pub struct PropsBuilder {
        #[generic_with_constraint = "SigT<Item = f32>"]
        #[generic_name = "P"]
        period_s: _,
        #[generic_with_constraint = "SigT<Item = f32>"]
        #[generic_name = "M"]
        #[default = 0.5]
        mix_01: f32,
        #[generic_with_constraint = "SigT<Item = f32>"]
        #[generic_name = "F"]
        #[default = 0.5]
        feedback_ratio: f32,
    }
}

pub struct DelayPeriodic<S, P, M, F>
where
    S: SigT<Item = f32>,
    P: SigT<Item = f32>,
    M: SigT<Item = f32>,
    F: SigT<Item = f32>,
{
    props: Props<P, M, F>,
    sig: S,
    ring: LinearlyInterpolatingRingBuffer,
    buf: Vec<f32>,
}

impl<P, M, F> Filter for PropsBuilder<P, M, F>
where
    P: SigT<Item = f32>,
    M: SigT<Item = f32>,
    F: SigT<Item = f32>,
{
    type ItemIn = f32;

    type Out<S>
        = DelayPeriodic<S, P, M, F>
    where
        S: SigT<Item = Self::ItemIn>;

    fn into_sig<S>(self, sig: S) -> Self::Out<S>
    where
        S: SigT<Item = f32>,
    {
        let props = self.build();
        DelayPeriodic {
            props,
            sig,
            ring: LinearlyInterpolatingRingBuffer::new(44100),
            buf: Vec::new(),
        }
    }
}

impl<S, P, M, F> SigT for DelayPeriodic<S, P, M, F>
where
    S: SigT<Item = f32>,
    P: SigT<Item = f32>,
    M: SigT<Item = f32>,
    F: SigT<Item = f32>,
{
    type Item = f32;

    fn sample(&mut self, ctx: &SigCtx) -> impl Buf<Self::Item> {
        self.buf.resize_with(ctx.num_samples, Default::default);
        for (out, sample, period_s, mix_01, feedback_ratio) in izip! {
            self.buf.iter_mut(),
            self.sig.sample(ctx).iter(),
            self.props.period_s.sample(ctx).iter(),
            self.props.mix_01.sample(ctx).iter(),
            self.props.feedback_ratio.sample(ctx).iter(),
        } {
            let index = period_s * ctx.sample_rate_hz;
            let output = self.ring.query_resizing(index);
            self.ring
                .insert((sample * mix_01) + (output * feedback_ratio));
            *out = output + (sample * (1.0 - mix_01));
        }
        &self.buf
    }
}
