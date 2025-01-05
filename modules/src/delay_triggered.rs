use caw_builder_proc_macros::builder;
use caw_core_next::{Buf, Filter, GateToTrigRisingEdge, Sig, SigCtx, SigT};
use itertools::izip;
use std::{collections::VecDeque, mem};

builder! {
    #[constructor = "delay_triggered"]
    #[constructor_doc = "Delay module where the signal repeats in response to a trigger"]
    #[generic_setter_type_name = "X"]
    pub struct Props {
        #[generic_with_constraint = "SigT<Item = bool>"]
        #[generic_name = "T"]
        trig: _,
        // 0 is dry signal, 1 is all delay
        #[generic_with_constraint = "SigT<Item = f32>"]
        #[generic_name = "M"]
        #[default = 0.5]
        mix_01: f32,
        // ratio of output fed back into the input
        #[generic_with_constraint = "SigT<Item = f32>"]
        #[generic_name = "F"]
        #[default = 0.5]
        feedback_ratio: f32,
    }
}

pub struct DelayTriggered<S, T, M, F>
where
    S: SigT<Item = f32>,
    T: SigT<Item = bool>,
    M: SigT<Item = f32>,
    F: SigT<Item = f32>,
{
    trig: GateToTrigRisingEdge<T>,
    // 0 is dry signal, 1 is all delay
    mix_01: M,
    // ratio of output fed back into the input
    feedback_ratio: F,
    sig: S,
    ring_write: VecDeque<S::Item>,
    ring_read: VecDeque<S::Item>,
    buf: Vec<S::Item>,
}

impl<T, M, F> Filter for Props<T, M, F>
where
    T: SigT<Item = bool>,
    M: SigT<Item = f32>,
    F: SigT<Item = f32>,
{
    type ItemIn = f32;

    type Out<S>
        = DelayTriggered<S, T, M, F>
    where
        S: SigT<Item = Self::ItemIn>;

    fn into_sig<S>(self, sig: S) -> Self::Out<S>
    where
        S: SigT<Item = Self::ItemIn>,
    {
        let trig = Sig(self.trig).gate_to_trig_rising_edge().0;
        DelayTriggered {
            trig,
            mix_01: self.mix_01,
            feedback_ratio: self.feedback_ratio,
            sig,
            ring_write: VecDeque::new(),
            ring_read: VecDeque::new(),
            buf: Vec::new(),
        }
    }
}

impl<S, T, M, F> SigT for DelayTriggered<S, T, M, F>
where
    S: SigT<Item = f32>,
    T: SigT<Item = bool>,
    M: SigT<Item = f32>,
    F: SigT<Item = f32>,
{
    type Item = S::Item;

    fn sample(&mut self, ctx: &SigCtx) -> impl Buf<Self::Item> {
        self.buf.resize_with(ctx.num_samples, Default::default);
        for (out, sample, trig, mix_01, feedback_ratio) in izip! {
            self.buf.iter_mut(),
            self.sig.sample(ctx).iter(),
            self.trig.sample(ctx).iter(),
            self.mix_01.sample(ctx).iter(),
            self.feedback_ratio.sample(ctx).iter(),
        } {
            if trig {
                mem::swap(&mut self.ring_read, &mut self.ring_write);
                self.ring_write.clear();
            }
            if let Some(output) = self.ring_read.pop_front() {
                *out = output + (sample * (1.0 - mix_01));
                self.ring_write
                    .push_back((sample * mix_01) + (output * feedback_ratio));
            } else {
                *out = sample;
                self.ring_write.push_back(sample * mix_01);
            }
        }

        &self.buf
    }
}
