use caw_builder_proc_macros::builder;
use caw_core_next::{Buf, Filter, SigCtx, SigT};
use itertools::izip;

pub struct Props<T>
where
    T: SigT<Item = bool>,
{
    trigger: T,
}

impl<T> Props<T>
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
    #[build_fn = "Props::new"]
    #[build_ty = "Props<T>"]
    #[generic_setter_type_name = "X"]
    pub struct PropsBuilder {
        #[generic_with_constraint = "SigT<Item = bool>"]
        #[generic_name = "T"]
        trigger: _,
    }
}

impl<T> Filter for PropsBuilder<T>
where
    T: SigT<Item = bool>,
{
    type ItemIn = f32;

    type Out<S> = SampleAndHold<S, T>
    where
        S: SigT<Item = Self::ItemIn>;

    fn into_sig<S>(self, sig: S) -> Self::Out<S>
    where
        S: SigT<Item = Self::ItemIn>,
    {
        let props = self.build();
        SampleAndHold {
            props,
            sig,
            value: Default::default(),
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
    props: Props<T>,
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
        for (out, sample, &trigger) in izip! {
            self.buf.iter_mut(),
            self.sig.sample(ctx).iter(),
            self.props.trigger.sample(ctx).iter(),
        } {
            if trigger {
                self.value = sample.clone();
            }
            *out = self.value.clone();
        }
        &self.buf
    }
}
