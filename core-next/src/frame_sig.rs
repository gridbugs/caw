use crate::{Buf, ConstBuf, Sig, SigCtx, SigT};
use std::{cell::RefCell, rc::Rc};

/// A signal with values produced each audio frame. This is distinct from the `SigT` trait whose
/// values are produced for each audio sample. Each audio frame corresponds to the sound driver
/// requesting a buffer of values. This is suitable for signals produced by input events such as a
/// mouse or computer keyboard, or midi keybeard.
pub trait FrameSigT {
    type Item: Clone;

    fn frame_sample(&mut self, ctx: &SigCtx) -> Self::Item;

    /// Convert `self` into a signal producing values at the audio sample rate, where values are
    /// duplicated within a given frame.
    fn into_sig(self) -> Sig<FrameSig<Self>>
    where
        Self: Sized,
    {
        Sig(FrameSig(self))
    }

    fn map<T, F>(self, f: F) -> FrameSig<Map<Self, T, F>>
    where
        T: Clone,
        Self: Sized,
        F: FnMut(Self::Item) -> T,
    {
        FrameSig(Map { sig: self, f })
    }

    fn map_ctx<T, F>(self, f: F) -> FrameSig<MapCtx<Self, T, F>>
    where
        T: Clone,
        Self: Sized,
        F: FnMut(Self::Item, &SigCtx) -> T,
    {
        FrameSig(MapCtx { sig: self, f })
    }

    fn zip<O>(self, other: O) -> FrameSig<Zip<Self, O>>
    where
        Self: Sized,
        O: FrameSigT,
    {
        FrameSig(Zip { a: self, b: other })
    }

    fn shared(self) -> FrameSig<FrameSigShared<Self>>
    where
        Self: Sized,
    {
        FrameSig(FrameSigShared {
            shared_cached_sig: Rc::new(RefCell::new(FrameSigCached::new(self))),
        })
    }
}

pub struct FrameSigFn<F, T>(F)
where
    F: FnMut(&SigCtx) -> T,
    T: Clone;
impl<F, T> FrameSigT for FrameSigFn<F, T>
where
    F: FnMut(&SigCtx) -> T,
    T: Clone,
{
    type Item = T;

    fn frame_sample(&mut self, ctx: &SigCtx) -> Self::Item {
        (self.0)(ctx)
    }
}

/// Wrapper type for the `FrameSigT` trait to simplify some trait implementations for signals. For
/// example this allows arithmetic traits like `std::ops::Add` to be implemented for frame signals.
#[derive(Clone)]
pub struct FrameSig<S>(pub S)
where
    S: FrameSigT;

impl<F, T> FrameSig<FrameSigFn<F, T>>
where
    F: FnMut(&SigCtx) -> T,
    T: Clone,
{
    pub fn from_fn(f: F) -> Self {
        Self(FrameSigFn(f))
    }
}

impl<S> FrameSigT for FrameSig<S>
where
    S: FrameSigT,
{
    type Item = S::Item;

    fn frame_sample(&mut self, ctx: &SigCtx) -> Self::Item {
        self.0.frame_sample(ctx)
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
            value: self.0.frame_sample(ctx),
            count: ctx.num_samples,
        }
    }
}

impl FrameSigT for f32 {
    type Item = f32;

    fn frame_sample(&mut self, _ctx: &SigCtx) -> Self::Item {
        *self
    }
}

/// For convenience, allow ints to be used as frame signals, but still treat them as yielding
/// floats.
impl FrameSigT for i32 {
    type Item = f32;

    fn frame_sample(&mut self, _ctx: &SigCtx) -> Self::Item {
        *self as f32
    }
}

impl FrameSigT for bool {
    type Item = bool;

    fn frame_sample(&mut self, _ctx: &SigCtx) -> Self::Item {
        *self
    }
}

pub struct Map<S, T, F>
where
    S: FrameSigT,
    F: FnMut(S::Item) -> T,
{
    sig: S,
    f: F,
}

impl<S, T, F> FrameSigT for Map<S, T, F>
where
    T: Clone,
    S: FrameSigT,
    F: FnMut(S::Item) -> T,
{
    type Item = T;

    fn frame_sample(&mut self, ctx: &SigCtx) -> Self::Item {
        (self.f)(self.sig.frame_sample(ctx))
    }
}

pub struct MapCtx<S, T, F>
where
    S: FrameSigT,
    F: FnMut(S::Item, &SigCtx) -> T,
{
    sig: S,
    f: F,
}

impl<S, T, F> FrameSigT for MapCtx<S, T, F>
where
    T: Clone,
    S: FrameSigT,
    F: FnMut(S::Item, &SigCtx) -> T,
{
    type Item = T;

    fn frame_sample(&mut self, ctx: &SigCtx) -> Self::Item {
        (self.f)(self.sig.frame_sample(ctx), ctx)
    }
}

pub struct Zip<A, B>
where
    A: FrameSigT,
    B: FrameSigT,
{
    a: A,
    b: B,
}

impl<A, B> FrameSigT for Zip<A, B>
where
    A: FrameSigT,
    B: FrameSigT,
{
    type Item = (A::Item, B::Item);

    fn frame_sample(&mut self, ctx: &SigCtx) -> Self::Item {
        (self.a.frame_sample(ctx), self.b.frame_sample(ctx))
    }
}

/// Wrapper for a `Sig` that prevents recomputation of its value
/// for a particular point in time.
struct FrameSigCached<S>
where
    S: FrameSigT,
{
    sig: S,
    cache: Option<S::Item>,
    next_batch_index: u64,
}

impl<S> FrameSigCached<S>
where
    S: FrameSigT,
{
    fn new(sig: S) -> Self {
        Self {
            sig,
            cache: None,
            next_batch_index: 0,
        }
    }
}

/// A wrapper of a signal that can be shallow-cloned. It doesn't implement `SigT` that would be
/// less performant than iterating the underlying signal with a callback.
impl<S> FrameSigT for FrameSigCached<S>
where
    S: FrameSigT,
{
    type Item = S::Item;

    fn frame_sample(&mut self, ctx: &SigCtx) -> Self::Item {
        if ctx.batch_index >= self.next_batch_index {
            self.next_batch_index = ctx.batch_index + 1;
            self.cache = Some(self.sig.frame_sample(ctx));
        }
        self.cache.as_ref().unwrap().clone()
    }
}

pub struct FrameSigShared<S>
where
    S: FrameSigT,
{
    shared_cached_sig: Rc<RefCell<FrameSigCached<S>>>,
}

impl<S> Clone for FrameSigShared<S>
where
    S: FrameSigT,
{
    fn clone(&self) -> Self {
        FrameSigShared {
            shared_cached_sig: Rc::clone(&self.shared_cached_sig),
        }
    }
}

impl<S> FrameSigT for FrameSigShared<S>
where
    S: FrameSigT,
{
    type Item = S::Item;

    fn frame_sample(&mut self, ctx: &SigCtx) -> Self::Item {
        self.shared_cached_sig.borrow_mut().frame_sample(ctx)
    }
}
