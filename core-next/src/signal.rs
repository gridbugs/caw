use std::iter;

#[derive(Clone, Copy)]
pub struct SigCtx {
    pub sample_rate_hz: f32,
    pub batch_index: u64,
    pub num_samples: usize,
}

pub trait Buf<T> {
    fn iter<'a>(&'a self) -> impl Iterator<Item = &'a T>
    where
        T: 'a;
}

impl<T> Buf<T> for &Vec<T> {
    fn iter<'a>(&'a self) -> impl Iterator<Item = &'a T>
    where
        T: 'a,
    {
        (self as &[T]).iter()
    }
}

pub struct ConstBuf<T> {
    value: T,
    count: usize,
}

impl<T> Buf<T> for ConstBuf<T> {
    fn iter<'a>(&'a self) -> impl Iterator<Item = &'a T>
    where
        T: 'a,
    {
        iter::repeat_n(&self.value, self.count)
    }
}

pub trait SigT {
    type Item;

    fn sample(&mut self, ctx: &SigCtx) -> impl Buf<Self::Item>;

    fn map<T, F>(self, f: F) -> Sig<Map<Self, T, F>>
    where
        Self: Sized,
        Self::Item: Clone,
        F: FnMut(Self::Item) -> T,
    {
        Sig(Map {
            sig: self,
            f,
            buf: Vec::new(),
        })
    }

    fn zip<O>(self, other: O) -> Sig<Zip<Self, O>>
    where
        Self: Sized,
        O: SigT,
        Self::Item: Clone,
        O::Item: Clone,
    {
        Sig(Zip {
            a: self,
            b: other,
            buf: Vec::new(),
        })
    }
}

/// Wrapper type for the `SigT` trait to simplify some trait implementations for signals. For
/// example this allows arithmetic traits like `std::ops::Add` to be implemented for signals.
pub struct Sig<S>(pub S)
where
    S: SigT;

impl<S: SigT> SigT for Sig<S> {
    type Item = S::Item;

    fn sample(&mut self, ctx: &SigCtx) -> impl Buf<Self::Item> {
        self.0.sample(ctx)
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

pub struct Map<S, T, F>
where
    S: SigT,
    S::Item: Clone,
    F: FnMut(S::Item) -> T,
{
    sig: S,
    f: F,
    buf: Vec<T>,
}

impl<S, T, F> SigT for Map<S, T, F>
where
    S: SigT,
    S::Item: Clone,
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

pub struct Zip<A, B>
where
    A: SigT,
    B: SigT,
    A::Item: Clone,
    B::Item: Clone,
{
    a: A,
    b: B,
    buf: Vec<(A::Item, B::Item)>,
}

impl<A, B> SigT for Zip<A, B>
where
    A: SigT,
    B: SigT,
    A::Item: Clone,
    B::Item: Clone,
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

pub trait GateT: SigT<Item = bool> {}

pub trait TrigT: SigT<Item = bool> {}

#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub struct Never;

impl SigT for Never {
    type Item = bool;

    fn sample(&mut self, ctx: &SigCtx) -> impl Buf<Self::Item> {
        ConstBuf {
            value: false,
            count: ctx.num_samples,
        }
    }
}

impl TrigT for Never {}
