use crate::{Buf, ConstBuf, Sig, SigCtx, SigT};

/// A signal with values produced each audio frame. This is distinct from the `SigT` trait whose
/// values are produced for each audio sample. Each audio frame corresponds to the sound driver
/// requesting a buffer of values. This is suitable for signals produced by input events such as a
/// mouse or computer keyboard, or midi keybeard.
pub trait FrameSigT {
    type Item: Clone;

    fn sample(&mut self, ctx: &SigCtx) -> Self::Item;

    /// Convert `self` into a signal producing values at the audio sample rate, where values are
    /// duplicated within a given frame.
    fn into_sig(self) -> Sig<FrameSig<Self>>
    where
        Self: Sized,
    {
        Sig(FrameSig(self))
    }
}

/// Wrapper type for the `FrameSigT` trait to simplify some trait implementations for signals. For
/// example this allows arithmetic traits like `std::ops::Add` to be implemented for frame signals.
pub struct FrameSig<S>(S)
where
    S: FrameSigT;

impl<S> FrameSigT for FrameSig<S>
where
    S: FrameSigT,
{
    type Item = S::Item;

    fn sample(&mut self, ctx: &SigCtx) -> Self::Item {
        self.0.sample(ctx)
    }
}

/// Frame signals are also signals. This allows frame signals (such as the mouse position) to be
/// passed to functions accepting signals.
impl<S> SigT for FrameSig<S>
where
    S: FrameSigT,
{
    type Item = <S as FrameSigT>::Item;

    fn sample(&mut self, ctx: &SigCtx) -> impl Buf<Self::Item> {
        ConstBuf {
            value: self.0.sample(ctx),
            count: ctx.num_samples,
        }
    }
}

impl FrameSigT for f32 {
    type Item = f32;

    fn sample(&mut self, _ctx: &SigCtx) -> Self::Item {
        *self
    }
}

/// For convenience, allow ints to be used as frame signals, but still treat them as yielding
/// floats.
impl FrameSigT for i32 {
    type Item = f32;

    fn sample(&mut self, _ctx: &SigCtx) -> Self::Item {
        *self as f32
    }
}
