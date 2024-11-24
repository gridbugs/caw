use std::{cell::RefCell, fmt::Debug, iter, rc::Rc};

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
    fn iter<'a>(&'a self) -> impl Iterator<Item = &'a T>
    where
        T: 'a;

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
    fn iter<'a>(&'a self) -> impl Iterator<Item = &'a T>
    where
        T: 'a,
    {
        (self as &[T]).iter()
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
    fn iter<'a>(&'a self) -> impl Iterator<Item = &'a T>
    where
        T: 'a,
    {
        iter::repeat_n(&self.value, self.count)
    }

    fn clone_to_vec(&self, out: &mut Vec<T>) {
        out.resize_with(self.count, || self.value.clone());
        out.fill(self.value.clone());
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

    fn map<T, F>(self, f: F) -> Sig<Map<Self, T, F>>
    where
        T: Clone,
        Self: Sized,
        F: FnMut(Self::Item) -> T,
    {
        Sig(Map {
            sig: self,
            f,
            buf: Vec::new(),
        })
    }

    fn map_ctx<T, F>(self, f: F) -> Sig<MapCtx<Self, T, F>>
    where
        T: Clone,
        Self: Sized,
        F: FnMut(Self::Item, &SigCtx) -> T,
    {
        Sig(MapCtx {
            sig: self,
            f,
            buf: Vec::new(),
        })
    }

    fn zip<O>(self, other: O) -> Sig<Zip<Self, O>>
    where
        Self: Sized,
        O: SigT,
    {
        Sig(Zip {
            a: self,
            b: other,
            buf: Vec::new(),
        })
    }

    fn shared(self) -> Sig<SigShared<Self>>
    where
        Self: Sized,
    {
        Sig(SigShared {
            shared_cached_sig: Rc::new(RefCell::new(SigCached::new(self))),
            buf: Vec::new(),
        })
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
    S: SigT,
{
    pub fn debug<F: FnMut(&S::Item)>(
        self,
        mut f: F,
    ) -> Sig<impl SigT<Item = S::Item>> {
        self.map(move |x| {
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
    pub fn clamp_symetric<C>(
        self,
        max_unsigned: C,
    ) -> Sig<impl SigT<Item = f32>>
    where
        C: SigT<Item = f32>,
    {
        self.zip(max_unsigned).map(|(s, c)| {
            let c = c.abs();
            s.clamp(-c, c)
        })
    }
}

impl<S> Sig<S>
where
    S: SigT,
{
    pub fn filter<F>(self, filter: F) -> Sig<F::Out<S>>
    where
        F: Filter<ItemIn = S::Item>,
    {
        Sig(filter.into_sig(self.0))
    }
}

pub struct Map<S, T, F>
where
    S: SigT,
    S: SigT,
    F: FnMut(S::Item) -> T,
{
    sig: S,
    f: F,
    buf: Vec<T>,
}

impl<S, T, F> SigT for Map<S, T, F>
where
    T: Clone,
    S: SigT,
    F: FnMut(S::Item) -> T,
{
    type Item = T;

    fn sample(&mut self, ctx: &SigCtx) -> impl Buf<Self::Item> {
        let buf = self.sig.sample(ctx);
        self.buf.clear();
        self.buf.extend(buf.iter().cloned().map(&mut self.f));
        &self.buf
    }
}

pub struct MapCtx<S, T, F>
where
    S: SigT,
    F: FnMut(S::Item, &SigCtx) -> T,
{
    sig: S,
    f: F,
    buf: Vec<T>,
}

impl<S, T, F> SigT for MapCtx<S, T, F>
where
    T: Clone,
    S: SigT,
    F: FnMut(S::Item, &SigCtx) -> T,
{
    type Item = T;

    fn sample(&mut self, ctx: &SigCtx) -> impl Buf<Self::Item> {
        let buf = self.sig.sample(ctx);
        self.buf.clear();
        self.buf
            .extend(buf.iter().cloned().map(|x| (self.f)(x, ctx)));
        &self.buf
    }
}

pub struct Zip<A, B>
where
    A: SigT,
    B: SigT,
{
    a: A,
    b: B,
    buf: Vec<(A::Item, B::Item)>,
}

impl<A, B> SigT for Zip<A, B>
where
    A: SigT,
    B: SigT,
{
    type Item = (A::Item, B::Item);

    fn sample(&mut self, ctx: &SigCtx) -> impl Buf<Self::Item> {
        let buf_a = self.a.sample(ctx);
        let buf_b = self.b.sample(ctx);
        self.buf.clear();
        self.buf
            .extend(buf_a.iter().cloned().zip(buf_b.iter().cloned()));
        &self.buf
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
