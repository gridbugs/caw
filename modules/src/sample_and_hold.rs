use caw_builder_proc_macros::builder;
use caw_core_next::{Buf, Flt, Sig, SigCtx, SigT};
use itertools::izip;

pub struct SampleAndHoldFlt<T>
where
    T: SigT<Item = bool>,
{
    trigger: T,
}

impl<T> SampleAndHoldFlt<T>
where
    T: SigT<Item = bool>,
{
    fn new(trigger: T) -> Self {
        Self { trigger }
    }
}

builder! {
    #[constructor = "sample_and_hold"]
    #[constructor_doc = "Always yields the value of its input signal when the trigger was last true"]
    #[build_fn = "SampleAndHoldFlt::new"]
    #[build_ty = "SampleAndHoldFlt<T>"]
    #[generic_setter_type_name = "X"]
    pub struct SampleAndHoldFltBuilder {
        #[generic_with_constraint = "SigT<Item = bool>"]
        #[generic_name = "T"]
        trigger: _,
    }
}

impl<T> Flt for SampleAndHoldFlt<T>
where
    T: SigT<Item = bool>,
{
    type Out<S> = SampleAndHoldSig<S, T>
    where
        S: SigT<Item = f32>;

    fn into_sig<S>(self, sig: S) -> Self::Out<S>
    where
        S: SigT<Item = f32>,
    {
        SampleAndHoldSig {
            sig,
            trigger: self.trigger,
            value: Default::default(),
            buf: Vec::new(),
        }
    }
}

pub struct SampleAndHoldSig<S, T>
where
    S: SigT,
    S::Item: Clone + Default,
    T: SigT<Item = bool>,
{
    sig: S,
    trigger: T,
    value: S::Item,
    buf: Vec<S::Item>,
}

impl<S, T> SigT for SampleAndHoldSig<S, T>
where
    S: SigT,
    S::Item: Clone + Default,
    T: SigT<Item = bool>,
{
    type Item = S::Item;

    fn sample(&mut self, ctx: &SigCtx) -> impl Buf<Self::Item> {
        self.buf.resize_with(ctx.num_samples, Default::default);
        for (out, sample, &trigger) in izip! {
            self.buf.iter_mut(),
            self.sig.sample(ctx).iter(),
            self.trigger.sample(ctx).iter(),
        } {
            if trigger {
                self.value = sample.clone();
            }
            *out = self.value.clone();
        }
        &self.buf
    }
}
