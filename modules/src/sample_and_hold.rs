use caw_builder_proc_macros::builder;
use caw_core::{Buf, Filter, SigCtx, SigT};
use itertools::izip;

builder! {
    #[constructor = "sample_and_hold"]
    #[constructor_doc = "Always yields the value of its input signal when the trigger was last true"]
    #[generic_setter_type_name = "X"]
    pub struct Props {
        #[generic_with_constraint = "SigT<Item = bool>"]
        #[generic_name = "T"]
        trigger: _,
        #[generic_with_constraint = "Default"]
        #[generic_name = "V"]
        #[default = 0.0]
        initial_value: f32,
    }
}

impl<T, V> Filter for Props<T, V>
where
    T: SigT<Item = bool>,
    V: Default + Clone,
{
    type ItemIn = V;

    type Out<S>
        = SampleAndHold<S, T>
    where
        S: SigT<Item = Self::ItemIn>;

    fn into_sig<S>(self, sig: S) -> Self::Out<S>
    where
        S: SigT<Item = Self::ItemIn>,
    {
        let Props {
            trigger,
            initial_value,
        } = self;
        SampleAndHold {
            trigger,
            sig,
            value: initial_value,
            buf: Vec::new(),
        }
    }
}

pub struct SampleAndHold<S, T>
where
    S: SigT,
    S::Item: Clone + Default,
    T: SigT<Item = bool>,
{
    trigger: T,
    sig: S,
    value: S::Item,
    buf: Vec<S::Item>,
}

impl<S, T> SigT for SampleAndHold<S, T>
where
    S: SigT,
    S::Item: Clone + Default,
    T: SigT<Item = bool>,
{
    type Item = S::Item;

    fn sample(&mut self, ctx: &SigCtx) -> impl Buf<Self::Item> {
        self.buf.resize_with(ctx.num_samples, Default::default);
        let sig = self.sig.sample(ctx);
        let trigger = self.trigger.sample(ctx);
        for (out, sample, trigger) in izip! {
            self.buf.iter_mut(),
            sig.iter(),
            trigger.iter(),
        } {
            if trigger {
                self.value = sample;
            }
            *out = self.value.clone();
        }
        &self.buf
    }
}
