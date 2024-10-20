use std::{cell::RefCell, rc::Rc};

pub struct SignalCtx {
    pub sample_rate_hz: f64,
    pub batch_index: u64,
}

pub trait SampleBuffer<T>: Default {
    fn iter<'a>(&'a self) -> impl Iterator<Item = &'a T>
    where
        T: 'a;

    fn clear(&mut self);
}

impl<T> SampleBuffer<T> for Vec<T> {
    fn iter<'a>(&'a self) -> impl Iterator<Item = &'a T>
    where
        T: 'a,
    {
        (self as &[T]).iter()
    }

    fn clear(&mut self) {
        Vec::clear(self)
    }
}

pub struct ConstSampleBuffer<T> {
    value: Option<T>,
    count: usize,
}

impl<T> Default for ConstSampleBuffer<T> {
    fn default() -> Self {
        Self {
            value: None,
            count: 0,
        }
    }
}

impl<T> SampleBuffer<T> for ConstSampleBuffer<T> {
    fn iter<'a>(&'a self) -> impl Iterator<Item = &'a T>
    where
        T: 'a,
    {
        self.value.iter().cycle().take(self.count)
    }

    fn clear(&mut self) {
        self.count = 0;
    }
}

pub struct BufferedSignal<S: SignalTrait> {
    pub signal: S,
    pub buffer: S::SampleBuffer,
}

impl<S: SignalTrait> BufferedSignal<S> {
    pub fn new(signal: S) -> Self {
        Self {
            signal,
            buffer: Default::default(),
        }
    }

    pub fn sample_batch(&mut self, ctx: &SignalCtx, n: usize) {
        self.buffer.clear();
        self.signal.sample_batch(ctx, n, &mut self.buffer);
    }

    pub fn samples(&self) -> impl Iterator<Item = &S::Item> {
        self.buffer.iter()
    }

    pub fn map<T, F>(self, f: F) -> BufferedSignal<Map<S, T, F>>
    where
        S: Sized,
        S::Item: Clone,
        F: FnMut(S::Item) -> T,
    {
        Map {
            buffered_signal: self,
            f,
        }
        .buffered()
    }

    pub fn map_ctx<T, F>(self, f: F) -> BufferedSignal<MapCtx<S, T, F>>
    where
        S: Sized,
        S::Item: Clone,
        F: FnMut(S::Item, &SignalCtx) -> T,
    {
        MapCtx {
            buffered_signal: self,
            f,
        }
        .buffered()
    }

    pub fn zip<O>(self, other: BufferedSignal<O>) -> BufferedSignal<Zip<S, O>>
    where
        S: Sized,
        O: SignalTrait,
        S::Item: Clone,
        O::Item: Clone,
    {
        Zip { a: self, b: other }.buffered()
    }

    pub fn add<R>(
        self,
        rhs: BufferedSignal<R>,
    ) -> BufferedSignal<
        impl SignalTrait<Item = <S::Item as std::ops::Add<R::Item>>::Output>,
    >
    where
        R: SignalTrait,
        S: Sized,
        S::Item: std::ops::Add<R::Item>,
        S::Item: Clone,
        R::Item: Clone,
    {
        self.zip(rhs).map(|(s, r)| s + r)
    }
}

pub trait SignalTrait {
    type Item;
    type SampleBuffer: SampleBuffer<Self::Item>;

    fn sample_batch(
        &mut self,
        ctx: &SignalCtx,
        n: usize,
        sample_buffer: &mut Self::SampleBuffer,
    );

    fn buffered(self) -> BufferedSignal<Self>
    where
        Self: Sized,
    {
        BufferedSignal::new(self)
    }

    /// Returns a `SignalTrait` with the same values as `self` but which
    /// avoids recomputing the value at each point in time.
    ///
    /// This returns an impl trait so that constant signals can
    /// override this method with a more efficient implementation.
    fn cached(self) -> impl SignalTrait<Item = Self::Item>
    where
        Self: Sized,
        Self::Item: Default + Clone,
    {
        SignalCached::new(self)
    }

    /// Returns a `SignalTrait` with the same values as `self` but which
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
    fn shared(self) -> impl SignalTrait<Item = Self::Item> + Clone
    where
        Self: Sized,
        Self::Item: Default + Clone,
    {
        SignalShared::new(self)
    }
}

pub trait GateTrait: SignalTrait<Item = bool> {
    fn to_trigger(self) -> impl TriggerTrait
    where
        Self: Sized,
    {
        GateToTrigger::new(self)
    }

    fn cached(self) -> impl GateTrait
    where
        Self: Sized,
    {
        SignalCached::new(self)
    }

    fn shared(self) -> impl GateTrait
    where
        Self: Sized,
    {
        SignalShared::new(self)
    }
}

pub trait TriggerTrait: SignalTrait<Item = bool> {
    fn cached(self) -> impl TriggerTrait
    where
        Self: Sized,
    {
        SignalCached::new(self)
    }

    fn shared(self) -> impl TriggerTrait
    where
        Self: Sized,
    {
        SignalShared::new(self)
    }
}

pub struct Map<S, T, F>
where
    S: SignalTrait,
    S::Item: Clone,
    F: FnMut(S::Item) -> T,
{
    buffered_signal: BufferedSignal<S>,
    f: F,
}

impl<S, T, F> SignalTrait for Map<S, T, F>
where
    S: SignalTrait,
    S::Item: Clone,
    F: FnMut(S::Item) -> T,
{
    type Item = T;
    type SampleBuffer = Vec<Self::Item>;

    fn sample_batch(
        &mut self,
        ctx: &SignalCtx,
        n: usize,
        sample_buffer: &mut Self::SampleBuffer,
    ) {
        self.buffered_signal.sample_batch(ctx, n);
        for sample in self.buffered_signal.samples() {
            sample_buffer.push((self.f)(sample.clone()));
        }
    }
}

pub struct MapCtx<S, T, F>
where
    S: SignalTrait,
    S::Item: Clone,
    F: FnMut(S::Item, &SignalCtx) -> T,
{
    buffered_signal: BufferedSignal<S>,
    f: F,
}

impl<S, T, F> SignalTrait for MapCtx<S, T, F>
where
    S: SignalTrait,
    S::Item: Clone,
    F: FnMut(S::Item, &SignalCtx) -> T,
{
    type Item = T;
    type SampleBuffer = Vec<Self::Item>;

    fn sample_batch(
        &mut self,
        ctx: &SignalCtx,
        n: usize,
        sample_buffer: &mut Self::SampleBuffer,
    ) {
        self.buffered_signal.sample_batch(ctx, n);
        for sample in self.buffered_signal.samples() {
            sample_buffer.push((self.f)(sample.clone(), ctx));
        }
    }
}

pub struct Zip<A, B>
where
    A: SignalTrait,
    B: SignalTrait,
    A::Item: Clone,
    B::Item: Clone,
{
    a: BufferedSignal<A>,
    b: BufferedSignal<B>,
}

impl<A, B> SignalTrait for Zip<A, B>
where
    A: SignalTrait,
    B: SignalTrait,
    A::Item: Clone,
    B::Item: Clone,
{
    type Item = (A::Item, B::Item);
    type SampleBuffer = Vec<Self::Item>;

    fn sample_batch(
        &mut self,
        ctx: &SignalCtx,
        n: usize,
        sample_buffer: &mut Self::SampleBuffer,
    ) {
        self.a.sample_batch(ctx, n);
        self.b.sample_batch(ctx, n);
        for (a, b) in self.a.samples().zip(self.b.samples()) {
            sample_buffer.push((a.clone(), b.clone()))
        }
    }
}

/// Wrapper for a `SignalTrait` that prevents recomputation of its value
/// for a particular point in time.
struct SignalCached<S>
where
    S: SignalTrait,
    S::Item: Default + Clone,
{
    buffered_signal: BufferedSignal<S>,
    cache: Vec<S::Item>,
    next_batch_index: u64,
}

impl<S> SignalCached<S>
where
    S: SignalTrait,
    S::Item: Default + Clone,
{
    fn new(signal: S) -> Self {
        Self {
            buffered_signal: signal.buffered(),
            cache: Default::default(),
            next_batch_index: 0,
        }
    }
}

impl<S> SignalTrait for SignalCached<S>
where
    S: SignalTrait,
    S::Item: Default + Clone,
{
    type Item = S::Item;
    type SampleBuffer = Vec<Self::Item>;

    fn sample_batch(
        &mut self,
        ctx: &SignalCtx,
        n: usize,
        sample_buffer: &mut Self::SampleBuffer,
    ) {
        if ctx.batch_index < self.next_batch_index {
            sample_buffer.clone_from_slice(&self.cache);
        } else {
            self.next_batch_index = ctx.batch_index + 1;
            self.buffered_signal.sample_batch(ctx, n);
            self.cache.clear();
            for sample in self.buffered_signal.samples() {
                self.cache.push(sample.clone());
                sample_buffer.push(sample.clone());
            }
        }
    }
}

impl<S> GateTrait for SignalCached<S> where S: SignalTrait<Item = bool> {}
impl<S> TriggerTrait for SignalCached<S> where S: SignalTrait<Item = bool> {}

struct SignalShared<S: SignalTrait>(Rc<RefCell<SignalCached<S>>>)
where
    S::Item: Default + Clone;

impl<S> Clone for SignalShared<S>
where
    S: SignalTrait,
    S::Item: Default + Clone,
{
    fn clone(&self) -> Self {
        Self(Rc::clone(&self.0))
    }
}

impl<S> SignalShared<S>
where
    S: SignalTrait,
    S::Item: Default + Clone,
{
    fn new(signal: S) -> Self {
        Self(Rc::new(RefCell::new(SignalCached::new(signal))))
    }
}

impl<S> SignalTrait for SignalShared<S>
where
    S: SignalTrait,
    S::Item: Default + Clone,
{
    type Item = S::Item;
    type SampleBuffer = Vec<Self::Item>;

    fn sample_batch(
        &mut self,
        ctx: &SignalCtx,
        n: usize,
        sample_buffer: &mut Self::SampleBuffer,
    ) {
        self.0.borrow_mut().sample_batch(ctx, n, sample_buffer)
    }
}

impl<S> GateTrait for SignalShared<S> where S: SignalTrait<Item = bool> {}
impl<S> TriggerTrait for SignalShared<S> where S: SignalTrait<Item = bool> {}

struct GateToTrigger<G>
where
    G: GateTrait,
{
    previous: bool,
    buffered_gate: BufferedSignal<G>,
}

impl<G> GateToTrigger<G>
where
    G: GateTrait,
{
    fn new(gate: G) -> Self {
        Self {
            previous: false,
            buffered_gate: gate.buffered(),
        }
    }
}

impl<G> SignalTrait for GateToTrigger<G>
where
    G: GateTrait,
{
    type Item = bool;
    type SampleBuffer = Vec<Self::Item>;

    fn sample_batch(
        &mut self,
        ctx: &SignalCtx,
        n: usize,
        sample_buffer: &mut Self::SampleBuffer,
    ) {
        self.buffered_gate.sample_batch(ctx, n);
        for &sample in self.buffered_gate.samples() {
            let trigger_sample = sample && !self.previous;
            self.previous = sample;
            sample_buffer.push(trigger_sample);
        }
    }
}

impl<G> TriggerTrait for GateToTrigger<G> where G: GateTrait {}

impl<T, F> SignalTrait for F
where
    F: FnMut(&SignalCtx) -> T,
{
    type Item = T;
    type SampleBuffer = Vec<Self::Item>;

    fn sample_batch(
        &mut self,
        ctx: &SignalCtx,
        n: usize,
        sample_buffer: &mut Self::SampleBuffer,
    ) {
        for _ in 0..n {
            sample_buffer.push((self)(ctx));
        }
    }
}

impl<F> GateTrait for F where F: FnMut(&SignalCtx) -> bool {}

#[derive(Clone)]
pub struct Const<T>(T)
where
    T: Clone;

impl<T> SignalTrait for Const<T>
where
    T: Clone,
{
    type Item = T;
    type SampleBuffer = ConstSampleBuffer<Self::Item>;

    fn sample_batch(
        &mut self,
        _ctx: &SignalCtx,
        n: usize,
        sample_buffer: &mut Self::SampleBuffer,
    ) {
        *sample_buffer = ConstSampleBuffer {
            value: Some(self.0.clone()),
            count: n,
        };
    }

    fn cached(self) -> impl SignalTrait<Item = Self::Item> {
        self
    }

    fn shared(self) -> impl SignalTrait<Item = Self::Item> + Clone {
        self
    }
}

pub fn const_<T>(value: T) -> Const<T>
where
    T: Clone,
{
    Const(value)
}

impl SignalTrait for f64 {
    type Item = Self;
    type SampleBuffer = ConstSampleBuffer<Self::Item>;

    fn sample_batch(
        &mut self,
        _ctx: &SignalCtx,
        n: usize,
        sample_buffer: &mut Self::SampleBuffer,
    ) {
        *sample_buffer = ConstSampleBuffer {
            value: Some(*self),
            count: n,
        };
    }

    fn cached(self) -> impl SignalTrait<Item = Self::Item> {
        self
    }

    fn shared(self) -> impl SignalTrait<Item = Self::Item> + Clone {
        self
    }
}

impl SignalTrait for bool {
    type Item = Self;
    type SampleBuffer = ConstSampleBuffer<Self::Item>;

    fn sample_batch(
        &mut self,
        _ctx: &SignalCtx,
        n: usize,
        sample_buffer: &mut Self::SampleBuffer,
    ) {
        *sample_buffer = ConstSampleBuffer {
            value: Some(*self),
            count: n,
        };
    }

    fn cached(self) -> impl SignalTrait<Item = Self::Item> {
        self
    }

    fn shared(self) -> impl SignalTrait<Item = Self::Item> + Clone {
        self
    }
}

impl GateTrait for bool {}

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

impl SignalTrait for Freq {
    type Item = Self;
    type SampleBuffer = ConstSampleBuffer<Self::Item>;

    fn sample_batch(
        &mut self,
        _ctx: &SignalCtx,
        n: usize,
        sample_buffer: &mut Self::SampleBuffer,
    ) {
        *sample_buffer = ConstSampleBuffer {
            value: Some(*self),
            count: n,
        };
    }

    fn cached(self) -> impl SignalTrait<Item = Self::Item> {
        self
    }

    fn shared(self) -> impl SignalTrait<Item = Self::Item> + Clone {
        self
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub struct Never;

impl SignalTrait for Never {
    type Item = bool;
    type SampleBuffer = ConstSampleBuffer<Self::Item>;

    fn sample_batch(
        &mut self,
        _ctx: &SignalCtx,
        n: usize,
        sample_buffer: &mut Self::SampleBuffer,
    ) {
        *sample_buffer = ConstSampleBuffer {
            value: Some(false),
            count: n,
        };
    }

    fn cached(self) -> impl SignalTrait<Item = Self::Item> {
        self
    }

    fn shared(self) -> impl SignalTrait<Item = Self::Item> + Clone {
        self
    }
}

impl TriggerTrait for Never {}
