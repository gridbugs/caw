use crate::{Buf, ConstBuf, Filter, Sig, SigCtx, SigT};
use std::{cell::RefCell, fmt::Debug, rc::Rc};

/// A signal with values produced each audio frame. This is distinct from the `SigT` trait whose
/// values are produced for each audio sample. Each audio frame corresponds to the sound driver
/// requesting a buffer of values. This is suitable for signals produced by input events such as a
/// mouse or computer keyboard, or midi keybeard.
pub trait FrameSigT {
    type Item: Clone;

    fn frame_sample(&mut self, ctx: &SigCtx) -> Self::Item;
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

impl<S> FrameSig<S>
where
    S: FrameSigT,
{
    /// Convert `self` into a signal producing values at the audio sample rate, where values are
    /// duplicated within a given frame.
    pub fn into_sig(self) -> Sig<Self> {
        Sig(self)
    }

    pub fn map<T, F>(self, f: F) -> FrameSig<Map<S, T, F>>
    where
        T: Clone,
        F: FnMut(S::Item) -> T,
    {
        FrameSig(Map { sig: self.0, f })
    }

    pub fn map_ctx<T, F>(self, f: F) -> FrameSig<MapCtx<S, T, F>>
    where
        T: Clone,
        F: FnMut(S::Item, &SigCtx) -> T,
    {
        FrameSig(MapCtx { sig: self.0, f })
    }

    pub fn zip<O>(self, other: O) -> FrameSig<Zip<Self, O>>
    where
        Self: Sized,
        O: FrameSigT,
    {
        FrameSig(Zip { a: self, b: other })
    }

    pub fn filter<F>(self, filter: F) -> Sig<F::Out<Sig<Self>>>
    where
        F: Filter<ItemIn = S::Item>,
    {
        self.into_sig().filter(filter)
    }

    pub fn shared(self) -> FrameSig<FrameSigShared<S>> {
        frame_sig_shared(self.0)
    }

    pub fn debug<F: FnMut(&S::Item)>(
        self,
        mut f: F,
    ) -> FrameSig<impl FrameSigT<Item = S::Item>> {
        self.map(move |x| {
            f(&x);
            x
        })
    }
}

impl<S> FrameSig<S>
where
    S: FrameSigT + 'static,
{
    /// Erase the type information about self so multiple different frame signals can be stored in
    /// a collection.
    pub fn boxed(self) -> FrameSig<FrameSigBoxed<S::Item>> {
        FrameSig(FrameSigBoxed(Box::new(self)))
    }
}

impl<S> FrameSig<S>
where
    S: FrameSigT<Item: Debug>,
{
    pub fn debug_print(self) -> FrameSig<impl FrameSigT<Item = S::Item>> {
        self.debug(|x| println!("{:?}", x))
    }
}

impl<S> FrameSig<S>
where
    S: FrameSigT<Item = f32>,
{
    /// clamp `x` between +/- `max_unsigned`
    pub fn clamp_symetric<C>(
        self,
        max_unsigned: C,
    ) -> FrameSig<impl FrameSigT<Item = f32>>
    where
        C: FrameSigT<Item = f32>,
    {
        self.zip(max_unsigned).map(|(s, max_unsigned)| {
            crate::arith::clamp_symetric(s, max_unsigned)
        })
    }

    /// The function f(x) =
    ///   k > 0  => exp(k * (x - a)) - b
    ///   k == 0 => x
    ///   k < 0  => -(ln(x + b) / k) + a
    /// ...where a and b are chosen so that f(0) = 0 and f(1) = 1.
    /// The k parameter controls how sharp the curve is.
    /// The functions when k != 0 are inverses of each other and zip approach linearity as k
    /// approaches 0.
    pub fn exp_01<K>(self, k: K) -> FrameSig<impl FrameSigT<Item = f32>>
    where
        K: FrameSigT<Item = f32>,
    {
        self.zip(k).map(|(x, k)| crate::arith::exp_01(x, k))
    }

    pub fn inv_01(self) -> FrameSig<impl FrameSigT<Item = f32>> {
        1.0 - self
    }

    pub fn signed_to_01(self) -> FrameSig<impl FrameSigT<Item = f32>> {
        (self + 1.0) / 2.0
    }
}

impl<S> FrameSig<S>
where
    S: FrameSigT<Item = bool>,
{
    pub fn on<T, F>(
        self,
        mut f: F,
    ) -> FrameSig<impl FrameSigT<Item = Option<T>>>
    where
        T: Clone,
        F: FnMut() -> T,
    {
        self.map(move |x| if x { Some(f()) } else { None })
    }

    pub fn on_unit<F>(self, mut f: F) -> FrameSig<impl FrameSigT<Item = ()>>
    where
        F: FnMut(),
    {
        self.map(move |x| {
            if x {
                f()
            }
        })
    }

    pub fn edges(self) -> FrameSigEdges<Self> {
        FrameSigEdges::new(self)
    }

    pub fn divide<B>(self, mut by: B) -> FrameSig<impl FrameSigT<Item = bool>>
    where
        B: FrameSigT<Item = u32>,
    {
        let mut edges = self.edges();
        let mut count = 0;
        FrameSig::from_fn(move |ctx| {
            let input = edges.frame_sample(ctx);
            // Increase the count on the falling edge rather than the rising edge so that the first
            // pulse is passed through regardless of the division.
            if edges.is_falling() {
                count += 1;
            }
            let by = by.frame_sample(ctx);
            if by == 0 || count % by == 0 {
                input
            } else {
                false
            }
        })
    }
}

pub struct FrameSigEdges<S>
where
    S: FrameSigT<Item = bool>,
{
    frame_sig: S,
    curr: bool,
    prev: bool,
}

impl<S> FrameSigT for FrameSigEdges<S>
where
    S: FrameSigT<Item = bool>,
{
    type Item = bool;

    fn frame_sample(&mut self, ctx: &SigCtx) -> Self::Item {
        self.prev = self.curr;
        self.curr = self.frame_sig.frame_sample(ctx);
        self.curr
    }
}

impl<S> FrameSigEdges<S>
where
    S: FrameSigT<Item = bool>,
{
    pub fn new(frame_sig: S) -> Self {
        Self {
            frame_sig,
            curr: false,
            prev: false,
        }
    }

    pub fn is_rising(&self) -> bool {
        self.curr && !self.prev
    }

    pub fn is_falling(&self) -> bool {
        !self.curr && self.prev
    }
}

struct OptionFirstSome<S>(Vec<S>)
where
    S: FrameSigT;

impl<T, S> FrameSigT for OptionFirstSome<S>
where
    T: Clone,
    S: FrameSigT<Item = Option<T>>,
{
    type Item = Option<T>;

    fn frame_sample(&mut self, ctx: &SigCtx) -> Self::Item {
        for s in self.0.iter_mut() {
            let x = s.frame_sample(ctx);
            if x.is_some() {
                return x;
            }
        }
        None
    }
}

impl<T, S> FrameSig<S>
where
    T: Clone,
    S: FrameSigT<Item = Option<T>>,
{
    pub fn option_or<O>(
        self,
        mut other: O,
    ) -> FrameSig<impl FrameSigT<Item = Option<T>>>
    where
        O: FrameSigT<Item = Option<T>>,
    {
        self.map_ctx(move |x, ctx| x.or(other.frame_sample(ctx)))
    }

    pub fn option_first_some(
        s: impl IntoIterator<Item = Self>,
    ) -> FrameSig<impl FrameSigT<Item = Option<T>>> {
        FrameSig(OptionFirstSome(s.into_iter().collect()))
    }
}

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

impl FrameSigT for u32 {
    type Item = u32;

    fn frame_sample(&mut self, _ctx: &SigCtx) -> Self::Item {
        *self
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

pub fn frame_sig_shared<S>(sig: S) -> FrameSig<FrameSigShared<S>>
where
    S: FrameSigT,
{
    FrameSig(FrameSigShared {
        shared_cached_sig: Rc::new(RefCell::new(FrameSigCached::new(sig))),
    })
}

pub struct FrameSigBoxed<T>(Box<dyn FrameSigT<Item = T>>);

impl<T> FrameSigT for FrameSigBoxed<T>
where
    T: Clone,
{
    type Item = T;

    fn frame_sample(&mut self, ctx: &SigCtx) -> Self::Item {
        self.0.frame_sample(ctx)
    }
}
