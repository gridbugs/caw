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
    ///
    /// This returns an impl trait so that constant signals can
    /// override this method with a more efficient implementation.
    fn cached(self) -> impl Signal<Item = Self::Item>
    where
        Self: Sized,
        Self::Item: Default + Clone,
    {
        SignalCached::new(self)
    }

    /// Returns a `Signal` with the same values as `self` but which
    /// can be cloned.
    ///
    /// For non-trivial implementations of signal, this is implemented
    /// by wrapping the signal in a `Rc<RefCell<_>>`, so a small
    /// performance cost will be incurred when sampling.
    ///
    /// Note that implementations of this method should produce
    /// signals that are cached as well as shared. That is, it should
    /// be unnecessary for callers to call `signal.cached().shared()`
    /// to produce a sharable value that avoids recomputation at a
    /// given point in time.
    ///
    /// This returns an impl trait so that constant signals can
    /// override this method with a more efficient implementation.
    fn shared(self) -> impl Signal<Item = Self::Item> + Clone
    where
        Self: Sized,
        Self::Item: Default + Clone,
    {
        SignalShared::new(self)
    }
}

pub trait Gate: Signal<Item = bool> {
    fn to_trigger(self) -> impl Trigger
    where
        Self: Sized,
    {
        GateToTrigger::new(self)
    }

    fn cached(self) -> impl Gate
    where
        Self: Sized,
    {
        SignalCached::new(self)
    }

    fn shared(self) -> impl Gate
    where
        Self: Sized,
    {
        SignalShared::new(self)
    }
}

pub trait Trigger: Signal<Item = bool> {
    fn cached(self) -> impl Trigger
    where
        Self: Sized,
    {
        SignalCached::new(self)
    }

    fn shared(self) -> impl Trigger
    where
        Self: Sized,
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
struct SignalCached<S>
where
    S: Signal,
    S::Item: Default + Clone,
{
    signal: S,
    buffered_sample: S::Item,
    next_sample_index: u64,
}

impl<S> SignalCached<S>
where
    S: Signal,
    S::Item: Default + Clone,
{
    fn new(signal: S) -> Self {
        Self {
            signal,
            buffered_sample: S::Item::default(),
            next_sample_index: 0,
        }
    }
}

impl<S> Signal for SignalCached<S>
where
    S: Signal,
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

impl<S> Gate for SignalCached<S> where S: Signal<Item = bool> {}
impl<S> Trigger for SignalCached<S> where S: Signal<Item = bool> {}

struct SignalShared<S: Signal>(Rc<RefCell<SignalCached<S>>>)
where
    S::Item: Default + Clone;

impl<S> Clone for SignalShared<S>
where
    S: Signal,
    S::Item: Default + Clone,
{
    fn clone(&self) -> Self {
        Self(Rc::clone(&self.0))
    }
}

impl<S> SignalShared<S>
where
    S: Signal,
    S::Item: Default + Clone,
{
    fn new(signal: S) -> Self {
        Self(Rc::new(RefCell::new(SignalCached::new(signal))))
    }
}

impl<S> Signal for SignalShared<S>
where
    S: Signal,
    S::Item: Default + Clone,
{
    type Item = S::Item;

    fn sample(&mut self, ctx: &SignalCtx) -> Self::Item {
        self.0.borrow_mut().sample(ctx)
    }
}

impl<S> Gate for SignalShared<S> where S: Signal<Item = bool> {}
impl<S> Trigger for SignalShared<S> where S: Signal<Item = bool> {}

struct GateToTrigger<G>
where
    G: Gate,
{
    previous: bool,
    gate: G,
}

impl<G> GateToTrigger<G>
where
    G: Gate,
{
    fn new(gate: G) -> Self {
        Self {
            previous: false,
            gate,
        }
    }
}

impl<G> Signal for GateToTrigger<G>
where
    G: Gate,
{
    type Item = bool;

    fn sample(&mut self, ctx: &SignalCtx) -> Self::Item {
        let sample = self.gate.sample(ctx);
        let trigger_sample = sample && !self.previous;
        self.previous = sample;
        trigger_sample
    }
}

impl<G> Trigger for GateToTrigger<G> where G: Gate {}

impl<T, F> Signal for F
where
    F: FnMut(&SignalCtx) -> T,
{
    type Item = T;
    fn sample(&mut self, ctx: &SignalCtx) -> Self::Item {
        (self)(ctx)
    }
}

impl<F> Gate for F where F: FnMut(&SignalCtx) -> bool {}

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

impl Signal for bool {
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

impl Gate for bool {}

#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub struct Freq {
    hz: f64,
}

impl Freq {
    pub const fn from_hz(hz: f64) -> Self {
        Self { hz }
    }

    pub const ZERO_HZ: Self = Self::from_hz(0.0);

    pub fn from_s(s: f64) -> Self {
        Self::from_hz(1.0 / s)
    }

    pub const fn hz(&self) -> f64 {
        self.hz
    }

    pub fn s(&self) -> f64 {
        self.hz() / 1.0
    }
}

pub const fn freq_hz(hz: f64) -> Freq {
    Freq::from_hz(hz)
}

pub fn freq_s(s: f64) -> Freq {
    Freq::from_s(s)
}

impl Signal for Freq {
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

#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub struct Never;

impl Signal for Never {
    type Item = bool;

    fn sample(&mut self, _ctx: &SignalCtx) -> Self::Item {
        false
    }

    fn cached(self) -> impl Signal<Item = Self::Item> {
        self
    }

    fn shared(self) -> impl Signal<Item = Self::Item> + Clone {
        self
    }
}

impl Trigger for Never {}
