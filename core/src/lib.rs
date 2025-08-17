mod arith;
mod sig;
pub mod sig_ops;
pub use sig::{
    Buf, Const, ConstBuf, Filter, GateToTrigRisingEdge, Sig, SigAbs, SigBoxed,
    SigBoxedVar, SigConst, SigCtx, SigSampleIntoBufT, SigShared, SigT, SigVar,
    Triggerable, Zip, Zip3, Zip4, sig_boxed, sig_boxed_var,
    sig_option_first_some, sig_shared, sig_var,
};
pub mod stereo;
pub use stereo::{Channel, Stereo, StereoPair};

pub type SV<T> = Sig<SigBoxedVar<T>>;
pub type SVF32 = SV<f32>;

pub fn sv<T>(initial_sig: impl SigT<Item = T> + Sync + Send + 'static) -> SV<T>
where
    T: Clone,
{
    sig_boxed_var(initial_sig)
}

pub fn svf32(
    initial_sig: impl SigT<Item = f32> + Sync + Send + 'static,
) -> SV<f32> {
    sig_boxed_var(initial_sig)
}

pub fn sv_default<T>() -> SV<T>
where
    T: SigT<Item = T> + Clone + Default + Sync + Send + 'static,
{
    sv(T::default())
}

impl<T> Stereo<SV<T>, SV<T>>
where
    T: Clone,
{
    pub fn set<F, S>(&self, mut f: F)
    where
        S: SigT<Item = T> + Sync + Send + 'static,
        F: FnMut() -> S,
    {
        self.as_ref().with(|s| s.set(f()));
    }

    pub fn set_channel<F, S>(&self, mut f: F)
    where
        S: SigT<Item = T> + Sync + Send + 'static,
        F: FnMut(Channel) -> S,
    {
        self.as_ref().with_channel(|channel, s| s.set(f(channel)));
    }
}

impl Stereo<SV<f32>, SV<f32>> {
    pub fn with_volume<V>(self, volume: V) -> Self
    where
        V: SigT<Item = f32> + Sync + Send + 'static,
    {
        let out = StereoPair::new_fn(|| sv(0.));
        let volume = sig_shared(volume);
        self.set_channel(|channel| out.get(channel).clone() * volume.clone());
        out
    }
}
