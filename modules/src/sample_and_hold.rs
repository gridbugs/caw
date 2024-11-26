use caw_builder_proc_macros::builder;
use caw_core_next::{Buf, Filter, SigCtx, SigT};
use itertools::izip;

pub struct Props<T, V>
where
    T: SigT<Item = bool>,
    V: Default,
{
    trigger: T,
    initial_value: V,
}

impl<T, V> Props<T, V>
where
    T: SigT<Item = bool>,
    V: Default,
{
    fn new(trigger: T, initial_value: V) -> Self {
        Self {
            trigger,
            initial_value,
        }
    }
}

builder! {
    #[constructor = "sample_and_hold"]
    #[constructor_doc = "Always yields the value of its input signal when the trigger was last true"]
    #[build_fn = "Props::new"]
    #[build_ty = "Props<T, V>"]
    #[generic_setter_type_name = "X"]
    pub struct PropsBuilder {
        #[generic_with_constraint = "SigT<Item = bool>"]
        #[generic_name = "T"]
        trigger: _,
        #[generic_with_constraint = "Default"]
        #[generic_name = "V"]
        #[default = 0.0]
        initial_value: f32,
    }
}

impl<T, V> Filter for PropsBuilder<T, V>
where
    T: SigT<Item = bool>,
    V: Default + Clone,
{
    type ItemIn = V;

    type Out<S> = SampleAndHold<S, T>
    where
        S: SigT<Item = Self::ItemIn>;

    fn into_sig<S>(self, sig: S) -> Self::Out<S>
    where
        S: SigT<Item = Self::ItemIn>,
    {
        let Props {
            trigger,
            initial_value,
        } = self.build();
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
        for (out, sample, trigger) in izip! {
            self.buf.iter_mut(),
            self.sig.sample(ctx).iter(),
            self.trigger.sample(ctx).iter(),
        } {
            if trigger {
                self.value = sample;
            }
            *out = self.value.clone();
        }
        &self.buf
    }
}
