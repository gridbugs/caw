use caw_builder_proc_macros::builder;
use caw_core_next::{Buf, Filter, SigCtx, SigT};
use itertools::izip;
use std::collections::VecDeque;

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
    ring: VecDeque<S::Item>,
    buf: Vec<S::Item>,
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
        S: SigT<Item = Self::ItemIn>,
    {
        let props = self.build();
        DelayPeriodic {
            props,
            sig,
            ring: VecDeque::new(),
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
    type Item = S::Item;

    fn sample(&mut self, ctx: &SigCtx) -> impl Buf<Self::Item> {
        self.buf.resize_with(ctx.num_samples, Default::default);
        for (out, sample, period_s, mix_01, feedback_ratio) in izip! {
            self.buf.iter_mut(),
            self.sig.sample(ctx).iter(),
            self.props.period_s.sample(ctx).iter(),
            self.props.mix_01.sample(ctx).iter(),
            self.props.feedback_ratio.sample(ctx).iter(),
        } {
            let new_len = (ctx.sample_rate_hz * period_s) as usize;
            if new_len == 0 {
                // Act like a no-op if the delay is 0.
                self.ring.clear();
                *out = sample;
            } else if new_len > self.ring.len() {
                // Grow the ring by adding a single element, and don't remove any elements.
                self.ring.push_back(sample * mix_01);
                *out = sample;
            } else {
                if new_len < self.ring.len() {
                    // Shrink the ring by removing excess elements from the back.
                    self.ring.resize(new_len, 0.);
                }
                // Remove the current output value from the ring before adding the new value. This
                // simplifies implementing feedback.
                let output = self.ring.pop_front().unwrap_or_default();
                // Scale the sample at the point where it's added to the ring rather than the point
                // where it's removed from the ring. This way the feedback ratio is applied
                // directly to the output without also applying the mix.
                self.ring
                    .push_back((sample * mix_01) + (output * feedback_ratio));
                *out = output + (sample * (1.0 - mix_01));
            }
        }

        &self.buf
    }
}
