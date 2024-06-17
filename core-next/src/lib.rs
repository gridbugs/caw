use std::{cell::RefCell, rc::Rc};

pub struct SignalCtx {
    pub sample_index: u64,
    pub sample_rate_hz: f64,
}

pub trait Signal {
    type Item;

    fn sample(&mut self, ctx: &SignalCtx) -> Self::Item;

    fn map<T, F>(self, f: F) -> Map<Self, T, F>
    where
        Self: Sized,
        F: FnMut(Self::Item) -> T,
    {
        Map { signal: self, f }
    }

    fn map_ctx<T, F>(self, f: F) -> MapCtx<Self, T, F>
    where
        Self: Sized,
        F: FnMut(Self::Item, &SignalCtx) -> T,
    {
        MapCtx { signal: self, f }
    }

    fn zip<S>(self, other: S) -> Zip<Self, S>
    where
        Self: Sized,
        S: Signal,
    {
        Zip { a: self, b: other }
    }

    /// Returns a `Signal` with the same values as `self` but which
    /// avoids recomputing the value at each point in time.
    fn cached(self) -> impl Signal<Item = Self::Item>
    where
        Self: Sized,
        Self::Item: Default + Clone,
    {
        SignalCached::new(self)
    }

    /// Returns a `Signal` with the same values as `self` but which
    /// can be cloned. For non-trivial implementations of signal, this
    /// is implemented by wrapping the signal in a `Rc<RefCell<_>>`,
    /// so a small performance cost will be incurred when
    /// sampling. Note that implementations of this method should
    /// produce signals that are cached as well as shared. That is, it
    /// should be unnecessary for callers to call
    /// `signal.cached().shared()` to produce a sharable value that
    /// avoids recomputation at a given point in time.
    fn shared(self) -> impl Signal<Item = Self::Item> + Clone
    where
        Self: Sized,
        Self::Item: Default + Clone,
    {
        SignalShared::new(self)
    }
}

pub struct Map<S, T, F>
where
    S: Signal,
    F: FnMut(S::Item) -> T,
{
    signal: S,
    f: F,
}

impl<S, T, F> Signal for Map<S, T, F>
where
    S: Signal,
    F: FnMut(S::Item) -> T,
{
    type Item = T;

    fn sample(&mut self, ctx: &SignalCtx) -> Self::Item {
        (self.f)(self.signal.sample(ctx))
    }
}

pub struct MapCtx<S, T, F>
where
    S: Signal,
    F: FnMut(S::Item, &SignalCtx) -> T,
{
    signal: S,
    f: F,
}

impl<S, T, F> Signal for MapCtx<S, T, F>
where
    S: Signal,
    F: FnMut(S::Item, &SignalCtx) -> T,
{
    type Item = T;

    fn sample(&mut self, ctx: &SignalCtx) -> Self::Item {
        (self.f)(self.signal.sample(ctx), ctx)
    }
}

pub struct Zip<A, B>
where
    A: Signal,
    B: Signal,
{
    a: A,
    b: B,
}

impl<A, B> Signal for Zip<A, B>
where
    A: Signal,
    B: Signal,
{
    type Item = (A::Item, B::Item);

    fn sample(&mut self, ctx: &SignalCtx) -> Self::Item {
        (self.a.sample(ctx), self.b.sample(ctx))
    }
}

/// Wrapper for a `Signal` that prevents recomputation of its value
/// for a particular point in time.
pub struct SignalCached<S: Signal>
where
    S::Item: Default + Clone,
{
    signal: S,
    buffered_sample: S::Item,
    next_sample_index: u64,
}

impl<S: Signal> SignalCached<S>
where
    S::Item: Default + Clone,
{
    pub fn new(signal: S) -> Self {
        Self {
            signal,
            buffered_sample: S::Item::default(),
            next_sample_index: 0,
        }
    }
}

impl<S: Signal> Signal for SignalCached<S>
where
    S::Item: Default + Clone,
{
    type Item = S::Item;

    fn sample(&mut self, ctx: &SignalCtx) -> Self::Item {
        if ctx.sample_index < self.next_sample_index {
            self.buffered_sample.clone()
        } else {
            self.next_sample_index = ctx.sample_index + 1;
            let sample = self.signal.sample(ctx);
            self.buffered_sample = sample.clone();
            sample
        }
    }
}

/// Wrapper for a signal that can be cloned and shared. This is
/// analagous to plugging multiple cables into a single output jack to
/// connect it to multiple input jacks.
pub struct SignalShared<S: Signal>(Rc<RefCell<SignalCached<S>>>)
where
    S::Item: Default + Clone;

impl<S: Signal> Clone for SignalShared<S>
where
    S::Item: Default + Clone,
{
    fn clone(&self) -> Self {
        Self(Rc::clone(&self.0))
    }
}

impl<S: Signal> SignalShared<S>
where
    S::Item: Default + Clone,
{
    pub fn new(signal: S) -> Self {
        Self(Rc::new(RefCell::new(SignalCached::new(signal))))
    }
}

impl<S: Signal> Signal for SignalShared<S>
where
    S::Item: Default + Clone,
{
    type Item = S::Item;

    fn sample(&mut self, ctx: &SignalCtx) -> Self::Item {
        self.0.borrow_mut().sample(ctx)
    }
}

impl<T, F> Signal for F
where
    F: FnMut(&SignalCtx) -> T,
{
    type Item = T;
    fn sample(&mut self, ctx: &SignalCtx) -> Self::Item {
        (self)(ctx)
    }
}

#[derive(Clone)]
pub struct Const<T>(T)
where
    T: Clone;

impl<T> Signal for Const<T>
where
    T: Clone,
{
    type Item = T;

    fn sample(&mut self, _ctx: &SignalCtx) -> Self::Item {
        self.0.clone()
    }

    fn cached(self) -> impl Signal<Item = Self::Item> {
        self
    }

    fn shared(self) -> impl Signal<Item = Self::Item> + Clone {
        self
    }
}

pub fn const_<T>(value: T) -> Const<T>
where
    T: Clone,
{
    Const(value)
}

#[derive(Default, Clone, Copy)]
pub enum Waveform {
    #[default]
    Sine,
}

impl Signal for Waveform {
    type Item = Self;
    fn sample(&mut self, _ctx: &SignalCtx) -> Self::Item {
        *self
    }
}

impl Signal for f64 {
    type Item = Self;
    fn sample(&mut self, _ctx: &SignalCtx) -> Self::Item {
        *self
    }

    fn cached(self) -> impl Signal<Item = Self::Item> {
        self
    }

    fn shared(self) -> impl Signal<Item = Self::Item> + Clone {
        self
    }
}
