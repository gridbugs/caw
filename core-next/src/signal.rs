use std::{cell::RefCell, rc::Rc};

#[derive(Clone, Copy)]
pub struct SigCtx {
    pub sample_rate_hz: f32,
    pub batch_index: u64,
    pub num_samples: usize,
}

pub trait Buf<T>: Default {
    fn iter<'a>(&'a self) -> impl Iterator<Item = &'a T>
    where
        T: 'a;

    fn clear(&mut self);
}

impl<T> Buf<T> for Vec<T> {
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

pub struct ConstBuf<T> {
    value: Option<T>,
    count: usize,
}

impl<T> Default for ConstBuf<T> {
    fn default() -> Self {
        Self {
            value: None,
            count: 0,
        }
    }
}

impl<T> Buf<T> for ConstBuf<T> {
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

pub struct SigBuf<S: Sig> {
    pub signal: S,
    pub buffer: S::Buf,
}

impl<S: Sig> SigBuf<S> {
    pub fn new(signal: S) -> Self {
        Self {
            signal,
            buffer: Default::default(),
        }
    }

    pub fn sample_batch(&mut self, ctx: &SigCtx) {
        self.buffer.clear();
        self.signal.sample_batch(ctx, &mut self.buffer);
    }

    pub fn samples(&self) -> impl Iterator<Item = &S::Item> {
        self.buffer.iter()
    }

    pub fn map<T, F>(self, f: F) -> SigBuf<Map<S, T, F>>
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

    pub fn map_ctx<T, F>(self, f: F) -> SigBuf<MapCtx<S, T, F>>
    where
        S: Sized,
        S::Item: Clone,
        F: FnMut(S::Item, &SigCtx) -> T,
    {
        MapCtx {
            buffered_signal: self,
            f,
        }
        .buffered()
    }

    pub fn zip<O>(self, other: SigBuf<O>) -> SigBuf<Zip<S, O>>
    where
        S: Sized,
        O: Sig,
        S::Item: Clone,
        O::Item: Clone,
    {
        Zip { a: self, b: other }.buffered()
    }

    pub fn add<R>(
        self,
        rhs: SigBuf<R>,
    ) -> SigBuf<impl Sig<Item = <S::Item as std::ops::Add<R::Item>>::Output>>
    where
        R: Sig,
        S: Sized,
        S::Item: std::ops::Add<R::Item>,
        S::Item: Clone,
        R::Item: Clone,
    {
        self.zip(rhs).map(|(s, r)| s + r)
    }
}

pub trait Sig {
    type Item;
    type Buf: Buf<Self::Item>;

    fn sample_batch(&mut self, ctx: &SigCtx, sample_buffer: &mut Self::Buf);

    fn buffered(self) -> SigBuf<Self>
    where
        Self: Sized,
    {
        SigBuf::new(self)
    }

    /// Returns a `Sig` with the same values as `self` but which
    /// avoids recomputing the value at each point in time.
    ///
    /// This returns an impl trait so that constant signals can
    /// override this method with a more efficient implementation.
    fn cached(self) -> impl Sig<Item = Self::Item>
    where
        Self: Sized,
        Self::Item: Default + Clone,
    {
        SigCached::new(self)
    }

    /// Returns a `Sig` with the same values as `self` but which
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
    fn shared(self) -> impl Sig<Item = Self::Item> + Clone
    where
        Self: Sized,
        Self::Item: Default + Clone,
    {
        SigShared::new(self)
    }
}

pub trait Gate: Sig<Item = bool> {
    fn to_trigger(self) -> impl Trig
    where
        Self: Sized,
    {
        GateToTrig::new(self)
    }

    fn cached(self) -> impl Gate
    where
        Self: Sized,
    {
        SigCached::new(self)
    }

    fn shared(self) -> impl Gate
    where
        Self: Sized,
    {
        SigShared::new(self)
    }
}

pub trait Trig: Sig<Item = bool> {
    fn cached(self) -> impl Trig
    where
        Self: Sized,
    {
        SigCached::new(self)
    }

    fn shared(self) -> impl Trig
    where
        Self: Sized,
    {
        SigShared::new(self)
    }
}

pub struct Map<S, T, F>
where
    S: Sig,
    S::Item: Clone,
    F: FnMut(S::Item) -> T,
{
    buffered_signal: SigBuf<S>,
    f: F,
}

impl<S, T, F> Sig for Map<S, T, F>
where
    S: Sig,
    S::Item: Clone,
    F: FnMut(S::Item) -> T,
{
    type Item = T;
    type Buf = Vec<Self::Item>;

    fn sample_batch(&mut self, ctx: &SigCtx, sample_buffer: &mut Self::Buf) {
        self.buffered_signal.sample_batch(ctx);
        for sample in self.buffered_signal.samples() {
            sample_buffer.push((self.f)(sample.clone()));
        }
    }
}

pub struct MapCtx<S, T, F>
where
    S: Sig,
    S::Item: Clone,
    F: FnMut(S::Item, &SigCtx) -> T,
{
    buffered_signal: SigBuf<S>,
    f: F,
}

impl<S, T, F> Sig for MapCtx<S, T, F>
where
    S: Sig,
    S::Item: Clone,
    F: FnMut(S::Item, &SigCtx) -> T,
{
    type Item = T;
    type Buf = Vec<Self::Item>;

    fn sample_batch(&mut self, ctx: &SigCtx, sample_buffer: &mut Self::Buf) {
        self.buffered_signal.sample_batch(ctx);
        for sample in self.buffered_signal.samples() {
            sample_buffer.push((self.f)(sample.clone(), ctx));
        }
    }
}

pub struct Zip<A, B>
where
    A: Sig,
    B: Sig,
    A::Item: Clone,
    B::Item: Clone,
{
    a: SigBuf<A>,
    b: SigBuf<B>,
}

impl<A, B> Sig for Zip<A, B>
where
    A: Sig,
    B: Sig,
    A::Item: Clone,
    B::Item: Clone,
{
    type Item = (A::Item, B::Item);
    type Buf = Vec<Self::Item>;

    fn sample_batch(&mut self, ctx: &SigCtx, sample_buffer: &mut Self::Buf) {
        self.a.sample_batch(ctx);
        self.b.sample_batch(ctx);
        for (a, b) in self.a.samples().zip(self.b.samples()) {
            sample_buffer.push((a.clone(), b.clone()))
        }
    }
}

/// Wrapper for a `Sig` that prevents recomputation of its value
/// for a particular point in time.
struct SigCached<S>
where
    S: Sig,
    S::Item: Default + Clone,
{
    buffered_signal: SigBuf<S>,
    cache: Vec<S::Item>,
    next_batch_index: u64,
}

impl<S> SigCached<S>
where
    S: Sig,
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

impl<S> Sig for SigCached<S>
where
    S: Sig,
    S::Item: Default + Clone,
{
    type Item = S::Item;
    type Buf = Vec<Self::Item>;

    fn sample_batch(&mut self, ctx: &SigCtx, sample_buffer: &mut Self::Buf) {
        if ctx.batch_index < self.next_batch_index {
            sample_buffer.clone_from_slice(&self.cache);
        } else {
            self.next_batch_index = ctx.batch_index + 1;
            self.buffered_signal.sample_batch(ctx);
            self.cache.clear();
            for sample in self.buffered_signal.samples() {
                self.cache.push(sample.clone());
                sample_buffer.push(sample.clone());
            }
        }
    }
}

impl<S> Gate for SigCached<S> where S: Sig<Item = bool> {}
impl<S> Trig for SigCached<S> where S: Sig<Item = bool> {}

struct SigShared<S: Sig>(Rc<RefCell<SigCached<S>>>)
where
    S::Item: Default + Clone;

impl<S> Clone for SigShared<S>
where
    S: Sig,
    S::Item: Default + Clone,
{
    fn clone(&self) -> Self {
        Self(Rc::clone(&self.0))
    }
}

impl<S> SigShared<S>
where
    S: Sig,
    S::Item: Default + Clone,
{
    fn new(signal: S) -> Self {
        Self(Rc::new(RefCell::new(SigCached::new(signal))))
    }
}

impl<S> Sig for SigShared<S>
where
    S: Sig,
    S::Item: Default + Clone,
{
    type Item = S::Item;
    type Buf = Vec<Self::Item>;

    fn sample_batch(&mut self, ctx: &SigCtx, sample_buffer: &mut Self::Buf) {
        self.0.borrow_mut().sample_batch(ctx, sample_buffer)
    }
}

impl<S> Gate for SigShared<S> where S: Sig<Item = bool> {}
impl<S> Trig for SigShared<S> where S: Sig<Item = bool> {}

struct GateToTrig<G>
where
    G: Gate,
{
    previous: bool,
    buffered_gate: SigBuf<G>,
}

impl<G> GateToTrig<G>
where
    G: Gate,
{
    fn new(gate: G) -> Self {
        Self {
            previous: false,
            buffered_gate: gate.buffered(),
        }
    }
}

impl<G> Sig for GateToTrig<G>
where
    G: Gate,
{
    type Item = bool;
    type Buf = Vec<Self::Item>;

    fn sample_batch(&mut self, ctx: &SigCtx, sample_buffer: &mut Self::Buf) {
        self.buffered_gate.sample_batch(ctx);
        for &sample in self.buffered_gate.samples() {
            let trigger_sample = sample && !self.previous;
            self.previous = sample;
            sample_buffer.push(trigger_sample);
        }
    }
}

impl<G> Trig for GateToTrig<G> where G: Gate {}

impl<T, F> Sig for F
where
    F: FnMut(&SigCtx) -> T,
{
    type Item = T;
    type Buf = Vec<Self::Item>;

    fn sample_batch(&mut self, ctx: &SigCtx, sample_buffer: &mut Self::Buf) {
        for _ in 0..ctx.num_samples {
            sample_buffer.push((self)(ctx));
        }
    }
}

impl<F> Gate for F where F: FnMut(&SigCtx) -> bool {}

#[derive(Clone)]
pub struct Const<T>(T)
where
    T: Clone;

impl<T> Sig for Const<T>
where
    T: Clone,
{
    type Item = T;
    type Buf = ConstBuf<Self::Item>;

    fn sample_batch(&mut self, ctx: &SigCtx, sample_buffer: &mut Self::Buf) {
        *sample_buffer = ConstBuf {
            value: Some(self.0.clone()),
            count: ctx.num_samples,
        };
    }

    fn cached(self) -> impl Sig<Item = Self::Item> {
        self
    }

    fn shared(self) -> impl Sig<Item = Self::Item> + Clone {
        self
    }
}

pub fn const_<T>(value: T) -> Const<T>
where
    T: Clone,
{
    Const(value)
}

impl Sig for f32 {
    type Item = Self;
    type Buf = ConstBuf<Self::Item>;

    fn sample_batch(&mut self, ctx: &SigCtx, sample_buffer: &mut Self::Buf) {
        *sample_buffer = ConstBuf {
            value: Some(*self),
            count: ctx.num_samples,
        };
    }

    fn cached(self) -> impl Sig<Item = Self::Item> {
        self
    }

    fn shared(self) -> impl Sig<Item = Self::Item> + Clone {
        self
    }
}

impl Sig for bool {
    type Item = Self;
    type Buf = ConstBuf<Self::Item>;

    fn sample_batch(&mut self, ctx: &SigCtx, sample_buffer: &mut Self::Buf) {
        *sample_buffer = ConstBuf {
            value: Some(*self),
            count: ctx.num_samples,
        };
    }

    fn cached(self) -> impl Sig<Item = Self::Item> {
        self
    }

    fn shared(self) -> impl Sig<Item = Self::Item> + Clone {
        self
    }
}

impl Gate for bool {}

#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub struct Freq {
    hz: f32,
}

impl Freq {
    pub const fn from_hz(hz: f32) -> Self {
        Self { hz }
    }

    pub const ZERO_HZ: Self = Self::from_hz(0.0);

    pub fn from_s(s: f32) -> Self {
        Self::from_hz(1.0 / s)
    }

    pub const fn hz(&self) -> f32 {
        self.hz
    }

    pub fn s(&self) -> f32 {
        self.hz() / 1.0
    }
}

pub const fn freq_hz(hz: f32) -> Freq {
    Freq::from_hz(hz)
}

pub fn freq_s(s: f32) -> Freq {
    Freq::from_s(s)
}

impl Sig for Freq {
    type Item = Self;
    type Buf = ConstBuf<Self::Item>;

    fn sample_batch(&mut self, ctx: &SigCtx, sample_buffer: &mut Self::Buf) {
        *sample_buffer = ConstBuf {
            value: Some(*self),
            count: ctx.num_samples,
        };
    }

    fn cached(self) -> impl Sig<Item = Self::Item> {
        self
    }

    fn shared(self) -> impl Sig<Item = Self::Item> + Clone {
        self
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub struct Never;

impl Sig for Never {
    type Item = bool;
    type Buf = ConstBuf<Self::Item>;

    fn sample_batch(&mut self, ctx: &SigCtx, sample_buffer: &mut Self::Buf) {
        *sample_buffer = ConstBuf {
            value: Some(false),
            count: ctx.num_samples,
        };
    }

    fn cached(self) -> impl Sig<Item = Self::Item> {
        self
    }

    fn shared(self) -> impl Sig<Item = Self::Item> + Clone {
        self
    }
}

impl Trig for Never {}
