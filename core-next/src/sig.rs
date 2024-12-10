use std::{cell::RefCell, fmt::Debug, iter, marker::PhantomData, rc::Rc};

#[derive(Clone, Copy)]
pub struct SigCtx {
    pub sample_rate_hz: f32,
    pub batch_index: u64,
    pub num_samples: usize,
}

pub trait Buf<T>
where
    T: Clone,
{
    fn iter(&self) -> impl Iterator<Item = T>;

    fn clone_to_vec(&self, out: &mut Vec<T>) {
        out.clear();
        for x in self.iter() {
            out.push(x.clone());
        }
    }
}

impl<T> Buf<T> for &Vec<T>
where
    T: Clone,
{
    fn iter(&self) -> impl Iterator<Item = T> {
        (self as &[T]).iter().cloned()
    }

    fn clone_to_vec(&self, out: &mut Vec<T>) {
        if let Some(first) = self.first() {
            out.resize_with(self.len(), || first.clone());
            out.clone_from_slice(self);
        } else {
            // self is empty
            out.clear();
        }
    }
}

pub struct ConstBuf<T> {
    pub value: T,
    pub count: usize,
}

impl<T> Buf<T> for ConstBuf<T>
where
    T: Clone,
{
    fn iter(&self) -> impl Iterator<Item = T> {
        iter::repeat_n(&self.value, self.count).cloned()
    }

    fn clone_to_vec(&self, out: &mut Vec<T>) {
        out.resize_with(self.count, || self.value.clone());
        out.fill(self.value.clone());
    }
}

/// Used to implement `map` methods (but not `map_mut`) by deferring the mapped function until the
/// iteration of the following operation, preventing the need to buffer the result of the map.
pub struct MapBuf<B, F, I, O>
where
    I: Clone,
    O: Clone,
    B: Buf<I>,
    F: Fn(I) -> O,
{
    buf: B,
    f: F,
    phantom: PhantomData<(I, O)>,
}

impl<B, F, I, O> MapBuf<B, F, I, O>
where
    I: Clone,
    O: Clone,
    B: Buf<I>,
    F: Fn(I) -> O,
{
    pub fn new(buf: B, f: F) -> Self {
        Self {
            buf,
            f,
            phantom: PhantomData,
        }
    }
}

impl<B, F, I, O> Buf<O> for MapBuf<B, F, I, O>
where
    I: Clone,
    O: Clone,
    B: Buf<I>,
    F: Fn(I) -> O,
{
    fn iter(&self) -> impl Iterator<Item = O> {
        self.buf.iter().map(&self.f)
    }
}

/// Used to implement arithmetic operations by deferring the computation until the iteration of the
/// following operation, preventing the need to buffer the result of the map.
pub struct MapBuf2<BL, BR, F, L, R, O>
where
    L: Clone,
    R: Clone,
    O: Clone,
    BL: Buf<L>,
    BR: Buf<R>,
    F: Fn(L, R) -> O,
{
    buf_left: BL,
    buf_right: BR,
    f: F,
    phantom: PhantomData<(L, R, O)>,
}

impl<BL, BR, F, L, R, O> MapBuf2<BL, BR, F, L, R, O>
where
    L: Clone,
    R: Clone,
    O: Clone,
    BL: Buf<L>,
    BR: Buf<R>,
    F: Fn(L, R) -> O,
{
    pub fn new(buf_left: BL, buf_right: BR, f: F) -> Self {
        Self {
            buf_left,
            buf_right,
            f,
            phantom: PhantomData,
        }
    }
}

impl<BL, BR, F, L, R, O> Buf<O> for MapBuf2<BL, BR, F, L, R, O>
where
    L: Clone,
    R: Clone,
    O: Clone,
    BL: Buf<L>,
    BR: Buf<R>,
    F: Fn(L, R) -> O,
{
    fn iter(&self) -> impl Iterator<Item = O> {
        self.buf_left
            .iter()
            .zip(self.buf_right.iter())
            .map(|(l, r)| (self.f)(l, r))
    }
}

/// Used to implement zip without the need for an explicit buffer for zipped values. The zipping
/// will take place in the iteration of the signal that consumes this buffer. Since zip operations
/// often follow map operations, and map operations use the `MapBuf` buffer type, sequences of zip
/// and map operations are fused without the need for intermediate buffers.
pub struct ZipBuf<BL, BR, L, R>
where
    L: Clone,
    R: Clone,
    BL: Buf<L>,
    BR: Buf<R>,
{
    buf_left: BL,
    buf_right: BR,
    phantom: PhantomData<(L, R)>,
}

impl<BL, BR, L, R> ZipBuf<BL, BR, L, R>
where
    L: Clone,
    R: Clone,
    BL: Buf<L>,
    BR: Buf<R>,
{
    pub fn new(buf_left: BL, buf_right: BR) -> Self {
        Self {
            buf_left,
            buf_right,
            phantom: PhantomData,
        }
    }
}

impl<BL, BR, L, R> Buf<(L, R)> for ZipBuf<BL, BR, L, R>
where
    L: Clone,
    R: Clone,
    BL: Buf<L>,
    BR: Buf<R>,
{
    fn iter(&self) -> impl Iterator<Item = (L, R)> {
        self.buf_left.iter().zip(self.buf_right.iter())
    }
}

/// A signal with values produced for each audio sample. Values are produced in batches of a size
/// determined by the audio driver. This is suitable for audible audio signals or controls signals
/// that vary at the same rate as an audio signal (e.g. an envelope follower produced by analyzing
/// an audio signal). Low-frequency signals such as LFOs should still use this type, as their
/// value still changes smoothly at the audio sample rate despite the signal they represent
/// typically being below perceptible audio frequencies.
pub trait SigT {
    type Item: Clone;

    fn sample(&mut self, ctx: &SigCtx) -> impl Buf<Self::Item>;

    fn filter<F>(self, filter: F) -> Sig<F::Out<Self>>
    where
        F: Filter<ItemIn = Self::Item>,
        Self: Sized,
    {
        Sig(filter.into_sig(self))
    }
}

impl SigT for f32 {
    type Item = f32;

    fn sample(&mut self, ctx: &SigCtx) -> impl Buf<Self::Item> {
        ConstBuf {
            value: *self,
            count: ctx.num_samples,
        }
    }
}

/// For convenience, allow ints to be used as signals, but still treat them as yielding floats.
impl SigT for i32 {
    type Item = f32;

    fn sample(&mut self, ctx: &SigCtx) -> impl Buf<Self::Item> {
        ConstBuf {
            value: *self as f32,
            count: ctx.num_samples,
        }
    }
}

/// For gate and trigger signals
impl SigT for bool {
    type Item = bool;

    fn sample(&mut self, ctx: &SigCtx) -> impl Buf<Self::Item> {
        ConstBuf {
            value: *self,
            count: ctx.num_samples,
        }
    }
}

pub struct SigConst<T: Clone>(T);
impl<T: Clone> SigT for SigConst<T> {
    type Item = T;

    fn sample(&mut self, ctx: &SigCtx) -> impl Buf<Self::Item> {
        ConstBuf {
            value: self.0.clone(),
            count: ctx.num_samples,
        }
    }
}

/// Wrapper type for the `SigT` trait to simplify some trait implementations for signals. For
/// example this allows arithmetic traits like `std::ops::Add` to be implemented for signals.
#[derive(Clone)]
pub struct Sig<S>(pub S)
where
    S: SigT;

impl<S: SigT> SigT for Sig<S> {
    type Item = S::Item;

    fn sample(&mut self, ctx: &SigCtx) -> impl Buf<Self::Item> {
        self.0.sample(ctx)
    }
}

impl<S> Sig<S>
where
    S: SigT<Item: Clone>,
{
    pub fn map_mut<T, F>(self, f: F) -> Sig<MapMut<S, T, F>>
    where
        T: Clone,
        F: FnMut(S::Item) -> T,
    {
        Sig(MapMut {
            sig: self.0,
            f,
            buf: Vec::new(),
        })
    }

    pub fn map_mut_ctx<T, F>(self, f: F) -> Sig<MapMutCtx<S, T, F>>
    where
        T: Clone,
        F: FnMut(S::Item, &SigCtx) -> T,
    {
        Sig(MapMutCtx {
            sig: self.0,
            f,
            buf: Vec::new(),
        })
    }

    pub fn map<T, F>(self, f: F) -> Sig<Map<S, T, F>>
    where
        T: Clone,
        F: Fn(S::Item) -> T,
    {
        Sig(Map { sig: self.0, f })
    }

    pub fn map_ctx<T, F>(self, f: F) -> Sig<MapCtx<S, T, F>>
    where
        T: Clone,
        F: Fn(S::Item, &SigCtx) -> T,
    {
        Sig(MapCtx { sig: self.0, f })
    }

    pub fn zip<O>(self, other: O) -> Sig<Zip<S, O>>
    where
        O: SigT,
    {
        Sig(Zip {
            a: self.0,
            b: other,
        })
    }

    pub fn shared(self) -> Sig<SigShared<S>> {
        sig_shared(self.0)
    }

    pub fn debug<F: FnMut(&S::Item)>(
        self,
        mut f: F,
    ) -> Sig<impl SigT<Item = S::Item>> {
        self.map_mut(move |x| {
            f(&x);
            x
        })
    }
}

impl<S> Sig<S>
where
    S: SigT<Item: Debug>,
{
    pub fn debug_print(self) -> Sig<impl SigT<Item = S::Item>> {
        self.debug(|x| println!("{:?}", x))
    }
}

impl<S> Sig<S>
where
    S: SigT<Item = f32>,
{
    /// clamp `x` between +/- `max_unsigned`
    pub fn clamp_symetric<C>(
        self,
        max_unsigned: C,
    ) -> Sig<impl SigT<Item = f32>>
    where
        C: SigT<Item = f32>,
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
    pub fn exp_01<K>(self, k: K) -> Sig<impl SigT<Item = f32>>
    where
        K: SigT<Item = f32>,
    {
        self.zip(k).map(|(x, k)| crate::arith::exp_01(x, k))
    }

    pub fn inv_01(self) -> Sig<impl SigT<Item = f32>> {
        1.0 - self
    }

    pub fn signed_to_01(self) -> Sig<impl SigT<Item = f32>> {
        (self + 1.0) / 2.0
    }

    pub fn abs(self) -> Sig<SigAbs<S>> {
        Sig(SigAbs(self.0))
    }
}

impl<S> Sig<S>
where
    S: SigT<Item = bool>,
{
    pub fn gate_to_trig_rising_edge(self) -> Sig<impl SigT<Item = bool>> {
        let mut previous = false;
        self.map_mut(move |x| {
            let out = x && !previous;
            previous = x;
            out
        })
    }

    pub fn trig_to_gate<P>(self, period_s: P) -> Sig<impl SigT<Item = bool>>
    where
        P: SigT<Item = f32>,
    {
        let mut remaining_s = 0.0;
        self.zip(period_s).map_mut_ctx(move |(x, period_s), ctx| {
            if x {
                remaining_s = period_s;
            }
            remaining_s -= 1.0 / ctx.sample_rate_hz;
            remaining_s > 0.0
        })
    }
}

pub struct MapMut<S, T, F>
where
    S: SigT,
    F: FnMut(S::Item) -> T,
{
    sig: S,
    f: F,
    buf: Vec<T>,
}

impl<S, T, F> SigT for MapMut<S, T, F>
where
    T: Clone,
    S: SigT,
    F: FnMut(S::Item) -> T,
{
    type Item = T;

    fn sample(&mut self, ctx: &SigCtx) -> impl Buf<Self::Item> {
        let buf = self.sig.sample(ctx);
        self.buf.clear();
        self.buf.extend(buf.iter().map(&mut self.f));
        &self.buf
    }
}

pub struct MapMutCtx<S, T, F>
where
    S: SigT,
    F: FnMut(S::Item, &SigCtx) -> T,
{
    sig: S,
    f: F,
    buf: Vec<T>,
}

impl<S, T, F> SigT for MapMutCtx<S, T, F>
where
    T: Clone,
    S: SigT,
    F: FnMut(S::Item, &SigCtx) -> T,
{
    type Item = T;

    fn sample(&mut self, ctx: &SigCtx) -> impl Buf<Self::Item> {
        let buf = self.sig.sample(ctx);
        self.buf.clear();
        self.buf.extend(buf.iter().map(|x| (self.f)(x, ctx)));
        &self.buf
    }
}

pub struct Map<S, T, F>
where
    S: SigT,
    F: Fn(S::Item) -> T,
{
    sig: S,
    f: F,
}

impl<S, T, F> SigT for Map<S, T, F>
where
    T: Clone,
    S: SigT,
    F: Fn(S::Item) -> T,
{
    type Item = T;

    fn sample(&mut self, ctx: &SigCtx) -> impl Buf<Self::Item> {
        MapBuf::new(self.sig.sample(ctx), &self.f)
    }
}

pub struct MapCtx<S, T, F>
where
    S: SigT,
    F: Fn(S::Item, &SigCtx) -> T,
{
    sig: S,
    f: F,
}

impl<S, T, F> SigT for MapCtx<S, T, F>
where
    T: Clone,
    S: SigT,
    F: Fn(S::Item, &SigCtx) -> T,
{
    type Item = T;

    fn sample(&mut self, ctx: &SigCtx) -> impl Buf<Self::Item> {
        MapBuf::new(self.sig.sample(ctx), |x| (self.f)(x, ctx))
    }
}

pub struct Zip<A, B>
where
    A: SigT,
    B: SigT,
{
    a: A,
    b: B,
}

impl<A, B> SigT for Zip<A, B>
where
    A: SigT,
    B: SigT,
{
    type Item = (A::Item, B::Item);

    fn sample(&mut self, ctx: &SigCtx) -> impl Buf<Self::Item> {
        ZipBuf::new(self.a.sample(ctx), self.b.sample(ctx))
    }
}

pub struct SigAbs<S>(S)
where
    S: SigT<Item = f32>;

impl<S> SigT for SigAbs<S>
where
    S: SigT<Item = f32>,
{
    type Item = f32;

    fn sample(&mut self, ctx: &SigCtx) -> impl Buf<Self::Item> {
        MapBuf::new(self.0.sample(ctx), |x| x.abs())
    }
}

/// For signals yielding `f32`, this trait provides a general way of defining filters.
pub trait Filter {
    /// The type of the item of the input signal to this filter.
    type ItemIn;

    /// The type of the signal produced by this filter. Filters take an input signal (`S`) and wrap
    /// them it a new signal whose type is this associated type. The output type will usually be
    /// generic with a type parameter for the input signal, so this associated type must also have
    /// that type parameter.
    type Out<S>: SigT
    where
        S: SigT<Item = Self::ItemIn>;

    /// Create a new signal from an existing signal, consuming self in the process.
    fn into_sig<S>(self, sig: S) -> Self::Out<S>
    where
        S: SigT<Item = Self::ItemIn>;
}

/// Wrapper for a `Sig` that prevents recomputation of its value
/// for a particular point in time.
struct SigCached<S>
where
    S: SigT,
{
    sig: S,
    cache: Vec<S::Item>,
    next_batch_index: u64,
}

impl<S> SigCached<S>
where
    S: SigT,
{
    fn new(sig: S) -> Self {
        Self {
            sig,
            cache: Vec::new(),
            next_batch_index: 0,
        }
    }
}

/// A wrapper of a signal that can be shallow-cloned. It doesn't implement `SigT` that would be
/// less performant than iterating the underlying signal with a callback.
impl<S> SigT for SigCached<S>
where
    S: SigT,
{
    type Item = S::Item;

    fn sample(&mut self, ctx: &SigCtx) -> impl Buf<Self::Item> {
        if ctx.batch_index >= self.next_batch_index {
            self.next_batch_index = ctx.batch_index + 1;
            let buf = self.sig.sample(ctx);
            buf.clone_to_vec(&mut self.cache);
        }
        &self.cache
    }
}

/// A wrapper of a signal which can be shallow-cloned. Use this to split a signal into two copies
/// of itself without duplicating all the computations that produced the signal. Incurs a small
/// performance penalty as buffered values must be copied from the underlying signal into each
/// instance of the shared signal.
pub struct SigShared<S>
where
    S: SigT,
{
    shared_cached_sig: Rc<RefCell<SigCached<S>>>,
    buf: Vec<S::Item>,
}

impl<S> Clone for SigShared<S>
where
    S: SigT,
{
    fn clone(&self) -> Self {
        SigShared {
            shared_cached_sig: Rc::clone(&self.shared_cached_sig),
            buf: self.buf.clone(),
        }
    }
}

impl<S> SigT for SigShared<S>
where
    S: SigT,
{
    type Item = S::Item;

    fn sample(&mut self, ctx: &SigCtx) -> impl Buf<Self::Item> {
        let mut shared_cached_sig = self.shared_cached_sig.borrow_mut();
        // This will only actually run the underlying signal if it hasn't been computed yet this
        // frame. If it has already been computed this frame then the already-populated buffer will
        // be returned. We still need to copy the buffer to the buffer inside `self` so that the
        // buffer returned by _this_ function has the appropriate lifetime.
        let buf = shared_cached_sig.sample(ctx);
        buf.clone_to_vec(&mut self.buf);
        &self.buf
    }
}

pub fn sig_shared<S>(sig: S) -> Sig<SigShared<S>>
where
    S: SigT,
{
    Sig(SigShared {
        shared_cached_sig: Rc::new(RefCell::new(SigCached::new(sig))),
        buf: Vec::new(),
    })
}
