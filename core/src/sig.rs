use crate::arith::signed_to_01;
use std::{
    fmt::Debug,
    iter,
    marker::PhantomData,
    sync::{Arc, RwLock},
};

#[derive(Clone, Copy, Debug)]
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

    /// Clears `out` and populates it with the contents of `self`.
    fn clone_to_vec(&self, out: &mut Vec<T>) {
        out.clear();
        for x in self.iter() {
            out.push(x.clone());
        }
    }

    /// Clone each sample into a slice with a given offset and stride. This is intended to be used
    /// to populate a single channel worth of samples into an audio buffer containing multiple
    /// interleaved channels.
    fn clone_to_slice(&self, stride: usize, offset: usize, out: &mut [T]) {
        let out_offset = &mut out[offset..];
        for (sample, out_chunk) in
            self.iter().zip(out_offset.chunks_mut(stride))
        {
            out_chunk[0] = sample;
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

/// Used to implement zip3 without the need for an explicit buffer for zipped values. The zipping
/// will take place in the iteration of the signal that consumes this buffer. Since zip operations
/// often follow map operations, and map operations use the `MapBuf` buffer type, sequences of zip
/// and map operations are fused without the need for intermediate buffers.
pub struct Zip3Buf<BA, BB, BC, A, B, C>
where
    A: Clone,
    B: Clone,
    C: Clone,
    BA: Buf<A>,
    BB: Buf<B>,
    BC: Buf<C>,
{
    buf_a: BA,
    buf_b: BB,
    buf_c: BC,
    phantom: PhantomData<(A, B, C)>,
}

impl<BA, BB, BC, A, B, C> Zip3Buf<BA, BB, BC, A, B, C>
where
    A: Clone,
    B: Clone,
    C: Clone,
    BA: Buf<A>,
    BB: Buf<B>,
    BC: Buf<C>,
{
    pub fn new(buf_a: BA, buf_b: BB, buf_c: BC) -> Self {
        Self {
            buf_a,
            buf_b,
            buf_c,
            phantom: PhantomData,
        }
    }
}

impl<BA, BB, BC, A, B, C> Buf<(A, B, C)> for Zip3Buf<BA, BB, BC, A, B, C>
where
    A: Clone,
    B: Clone,
    C: Clone,
    BA: Buf<A>,
    BB: Buf<B>,
    BC: Buf<C>,
{
    fn iter(&self) -> impl Iterator<Item = (A, B, C)> {
        self.buf_a
            .iter()
            .zip(self.buf_b.iter())
            .zip(self.buf_c.iter())
            .map(|((a, b), c)| (a, b, c))
    }
}

/// Used to implement zip4 without the need for an explicit buffer for zipped values. The zipping
/// will take place in the iteration of the signal that consumes this buffer. Since zip operations
/// often follow map operations, and map operations use the `MapBuf` buffer type, sequences of zip
/// and map operations are fused without the need for intermediate buffers.
pub struct Zip4Buf<BA, BB, BC, BD, A, B, C, D>
where
    A: Clone,
    B: Clone,
    C: Clone,
    D: Clone,
    BA: Buf<A>,
    BB: Buf<B>,
    BC: Buf<C>,
    BD: Buf<D>,
{
    buf_a: BA,
    buf_b: BB,
    buf_c: BC,
    buf_d: BD,
    phantom: PhantomData<(A, B, C, D)>,
}

impl<BA, BB, BC, BD, A, B, C, D> Zip4Buf<BA, BB, BC, BD, A, B, C, D>
where
    A: Clone,
    B: Clone,
    C: Clone,
    D: Clone,
    BA: Buf<A>,
    BB: Buf<B>,
    BC: Buf<C>,
    BD: Buf<D>,
{
    pub fn new(buf_a: BA, buf_b: BB, buf_c: BC, buf_d: BD) -> Self {
        Self {
            buf_a,
            buf_b,
            buf_c,
            buf_d,
            phantom: PhantomData,
        }
    }
}

impl<BA, BB, BC, BD, A, B, C, D> Buf<(A, B, C, D)>
    for Zip4Buf<BA, BB, BC, BD, A, B, C, D>
where
    A: Clone,
    B: Clone,
    C: Clone,
    D: Clone,
    BA: Buf<A>,
    BB: Buf<B>,
    BC: Buf<C>,
    BD: Buf<D>,
{
    fn iter(&self) -> impl Iterator<Item = (A, B, C, D)> {
        self.buf_a
            .iter()
            .zip(self.buf_b.iter())
            .zip(self.buf_c.iter())
            .zip(self.buf_d.iter())
            .map(|(((a, b), c), d)| (a, b, c, d))
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

/// Similar to `SigT` but less flexible as it needs to populate a `Vec`. However this trait is
/// possible to be boxed allowing for type erasure.
pub trait SigSampleIntoBufT {
    type Item: Clone;

    fn sample_into_buf(&mut self, ctx: &SigCtx, buf: &mut Vec<Self::Item>);

    fn sample_into_slice(
        &mut self,
        ctx: &SigCtx,
        stride: usize,
        offset: usize,
        out: &mut [Self::Item],
    );
}

pub struct Const<T>(T)
where
    T: Clone;

impl<T> SigT for Const<T>
where
    T: Clone,
{
    type Item = T;

    fn sample(&mut self, ctx: &SigCtx) -> impl Buf<Self::Item> {
        ConstBuf {
            value: self.0.clone(),
            count: ctx.num_samples,
        }
    }
}

impl SigT for f32 {
    type Item = Self;

    fn sample(&mut self, ctx: &SigCtx) -> impl Buf<Self::Item> {
        ConstBuf {
            value: *self,
            count: ctx.num_samples,
        }
    }
}

impl SigT for u32 {
    type Item = Self;

    fn sample(&mut self, ctx: &SigCtx) -> impl Buf<Self::Item> {
        ConstBuf {
            value: *self,
            count: ctx.num_samples,
        }
    }
}

/// For gate and trigger signals
impl SigT for bool {
    type Item = Self;

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

impl<S: SigT> SigSampleIntoBufT for Sig<S> {
    type Item = S::Item;

    fn sample_into_buf(&mut self, ctx: &SigCtx, buf: &mut Vec<Self::Item>) {
        let buf_internal = self.0.sample(ctx);
        buf_internal.clone_to_vec(buf);
    }

    fn sample_into_slice(
        &mut self,
        ctx: &SigCtx,
        stride: usize,
        offset: usize,
        out: &mut [Self::Item],
    ) {
        let buf_internal = self.0.sample(ctx);
        buf_internal.clone_to_slice(stride, offset, out);
    }
}

pub struct SigBoxed<T>
where
    T: Clone,
{
    sig: Arc<RwLock<dyn SigSampleIntoBufT<Item = T> + Send + Sync + 'static>>,
    buf: Vec<T>,
}

impl<T> Clone for SigBoxed<T>
where
    T: Clone,
{
    fn clone(&self) -> Self {
        SigBoxed {
            sig: Arc::clone(&self.sig),
            buf: Vec::new(),
        }
    }
}

impl<T> SigSampleIntoBufT for SigBoxed<T>
where
    T: Clone,
{
    type Item = T;

    fn sample_into_buf(&mut self, ctx: &SigCtx, buf: &mut Vec<Self::Item>) {
        self.sig.write().unwrap().sample_into_buf(ctx, buf);
    }

    fn sample_into_slice(
        &mut self,
        ctx: &SigCtx,
        stride: usize,
        offset: usize,
        out: &mut [Self::Item],
    ) {
        self.sig
            .write()
            .unwrap()
            .sample_into_slice(ctx, stride, offset, out);
    }
}

impl<T> SigT for SigBoxed<T>
where
    T: Clone,
{
    type Item = T;

    fn sample(&mut self, ctx: &SigCtx) -> impl Buf<Self::Item> {
        self.sig
            .write()
            .unwrap()
            .sample_into_buf(ctx, &mut self.buf);
        &self.buf
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

    pub fn for_each<F>(self, mut f: F) -> Sig<impl SigT<Item = S::Item>>
    where
        F: FnMut(S::Item),
    {
        self.map_mut(move |x| {
            f(x.clone());
            x
        })
    }

    /// Calls `f` once per frame on all the samples computed during that frame, returning the
    /// original signal unchanged.
    pub fn with_buf<F>(self, f: F) -> Sig<WithBuf<S, F>>
    where
        F: FnMut(&[S::Item]),
    {
        Sig(WithBuf {
            sig: self.0,
            f,
            buf: Vec::new(),
        })
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

    pub fn zip3<O1, O2>(self, other1: O1, other2: O2) -> Sig<Zip3<S, O1, O2>>
    where
        O1: SigT,
        O2: SigT,
    {
        Sig(Zip3 {
            a: self.0,
            b: other1,
            c: other2,
        })
    }

    pub fn zip4<O1, O2, O3>(
        self,
        other1: O1,
        other2: O2,
        other3: O3,
    ) -> Sig<Zip4<S, O1, O2, O3>>
    where
        O1: SigT,
        O2: SigT,
        O3: SigT,
    {
        Sig(Zip4 {
            a: self.0,
            b: other1,
            c: other2,
            d: other3,
        })
    }

    /// A signal whose items are None if the gate is down and the input signal if the gate is up.
    pub fn gated<G>(self, gate: G) -> Sig<impl SigT<Item = Option<S::Item>>>
    where
        G: SigT<Item = bool>,
    {
        self.zip(gate).map(
            |(self_, gate)| {
                if gate { Some(self_) } else { None }
            },
        )
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

    /// Force the evaluation of some other signal when this signal is evaluated but ignore the
    /// result. Use this when computing a signal has side effects (e.g. rendering a visualization)
    /// but the effectful signal's value is unnecessary.
    pub fn force<O>(self, other: O) -> Sig<impl SigT<Item = S::Item>>
    where
        O: SigT,
    {
        self.zip(other).map(|(x, _)| x)
    }
}

impl<S, A, B> Sig<S>
where
    S: SigT<Item = (A, B)>,
    A: Clone,
    B: Clone,
{
    pub fn unzip(self) -> (Sig<impl SigT<Item = A>>, Sig<impl SigT<Item = B>>) {
        let shared = self.shared();
        let a = shared.clone().map(|(a, _)| a);
        let b = shared.clone().map(|(_, b)| b);
        (a, b)
    }
}

impl<S, A, B, C> Sig<S>
where
    S: SigT<Item = (A, B, C)>,
    A: Clone,
    B: Clone,
    C: Clone,
{
    pub fn unzip3(
        self,
    ) -> (
        Sig<impl SigT<Item = A>>,
        Sig<impl SigT<Item = B>>,
        Sig<impl SigT<Item = C>>,
    ) {
        let shared = self.shared();
        let a = shared.clone().map(|(a, _, _)| a);
        let b = shared.clone().map(|(_, b, _)| b);
        let c = shared.clone().map(|(_, _, c)| c);
        (a, b, c)
    }
}

impl<S, A, B, C, D> Sig<S>
where
    S: SigT<Item = (A, B, C, D)>,
    A: Clone,
    B: Clone,
    C: Clone,
    D: Clone,
{
    pub fn unzip4(
        self,
    ) -> (
        Sig<impl SigT<Item = A>>,
        Sig<impl SigT<Item = B>>,
        Sig<impl SigT<Item = C>>,
        Sig<impl SigT<Item = D>>,
    ) {
        let shared = self.shared();
        let a = shared.clone().map(|(a, _, _, _)| a);
        let b = shared.clone().map(|(_, b, _, _)| b);
        let c = shared.clone().map(|(_, _, c, _)| c);
        let d = shared.clone().map(|(_, _, _, d)| d);
        (a, b, c, d)
    }
}

pub fn sig_boxed<S>(sig: S) -> SigBoxed<S::Item>
where
    S: SigT + Send + Sync + 'static,
{
    Sig(sig).boxed()
}

impl<S> Sig<S>
where
    S: SigT + Send + Sync + 'static,
{
    pub fn boxed(self) -> SigBoxed<S::Item> {
        SigBoxed {
            sig: Arc::new(RwLock::new(self)),
            buf: Vec::new(),
        }
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

    pub fn signed_to_01(self) -> Sig<SignedTo01<S>> {
        Sig(SignedTo01(self.0))
    }

    pub fn abs(self) -> Sig<SigAbs<S>> {
        Sig(SigAbs(self.0))
    }

    pub fn is_positive(self) -> Sig<impl SigT<Item = bool>> {
        self.map(|x| x > 0.0)
    }

    pub fn is_negative(self) -> Sig<impl SigT<Item = bool>> {
        self.map(|x| x < 0.0)
    }
}

pub struct GateToTrigRisingEdge<S>
where
    S: SigT<Item = bool>,
{
    sig: S,
    prev: bool,
    buf: Vec<bool>,
}

impl<S> SigT for GateToTrigRisingEdge<S>
where
    S: SigT<Item = bool>,
{
    type Item = bool;

    fn sample(&mut self, ctx: &SigCtx) -> impl Buf<Self::Item> {
        self.buf.resize(ctx.num_samples, false);
        for (out, sample) in
            self.buf.iter_mut().zip(self.sig.sample(ctx).iter())
        {
            *out = sample && !self.prev;
            self.prev = sample;
        }
        &self.buf
    }
}

impl<S> Sig<S>
where
    S: SigT<Item = bool>,
{
    pub fn gate_to_trig_rising_edge(self) -> Sig<GateToTrigRisingEdge<S>> {
        Sig(GateToTrigRisingEdge {
            sig: self.0,
            prev: false,
            buf: Vec::new(),
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

    pub fn trig<T>(self, triggerable: T) -> Sig<impl SigT<Item = T::Item>>
    where
        T: Triggerable,
    {
        Sig(triggerable.into_sig(self))
    }

    pub fn on<T, F>(self, mut f: F) -> Sig<impl SigT<Item = Option<T>>>
    where
        T: Clone,
        F: FnMut() -> T,
    {
        self.map_mut(move |x| if x { Some(f()) } else { None })
    }

    pub fn divide_with_offset<D, O>(
        self,
        divide: D,
        offset: O,
    ) -> Sig<impl SigT<Item = bool>>
    where
        D: SigT<Item = u32>,
        O: SigT<Item = u32>,
    {
        let mut prev = false;
        let mut count = 0u64;
        self.zip3(divide, offset)
            .map_mut(move |(current, divide, offset)| {
                let divide = divide as u64;
                let offset = offset as u64;
                let is_falling_edge = prev && !current;
                if is_falling_edge {
                    count += 1;
                }
                prev = current;
                if divide == 0 {
                    // degenerate case
                    true
                } else if count < offset {
                    // first few tick until count overtakes the offset
                    false
                } else if (count - offset) % divide == 0 {
                    current
                } else {
                    false
                }
            })
    }

    pub fn divide<B>(self, by: B) -> Sig<impl SigT<Item = bool>>
    where
        B: SigT<Item = u32>,
    {
        self.divide_with_offset(by, 0)
    }

    pub fn counted(self) -> Sig<impl SigT<Item = Option<u64>>> {
        let mut prev = false;
        let mut count = 0;
        self.map_mut(move |current| {
            let ret = if current {
                Some(count)
            } else {
                if prev {
                    // increment the count on the falling edge
                    count += 1;
                }
                None
            };
            prev = current;
            ret
        })
    }
}

/// A counting gate, which is a gate which implements a counter each time it transitions to "on".
impl<S> Sig<S>
where
    S: SigT<Item = Option<u64>>,
{
    pub fn counted_to_gate(self) -> Sig<impl SigT<Item = bool>> {
        self.map(|x| x.is_some())
    }
    pub fn counted_to_trig_rising_edge(self) -> Sig<impl SigT<Item = bool>> {
        self.counted_to_gate().gate_to_trig_rising_edge()
    }
    pub fn counted_divide_with_offset<D, O>(
        self,
        divide: D,
        offset: O,
    ) -> Sig<impl SigT<Item = bool>>
    where
        D: SigT<Item = u32>,
        O: SigT<Item = u32>,
    {
        self.zip3(divide, offset).map(|(current, divide, offset)| {
            if let Some(current) = current {
                let divide = divide as u64;
                let offset = offset as u64;
                if divide == 0 {
                    // degenerate case
                    true
                } else {
                    current % divide == offset
                }
            } else {
                false
            }
        })
    }
    pub fn counted_divide<B>(self, by: B) -> Sig<impl SigT<Item = bool>>
    where
        B: SigT<Item = u32>,
    {
        self.counted_divide_with_offset(by, 0)
    }
}

struct OptionFirstSome<T>
where
    T: Clone,
{
    sigs: Vec<SigBoxed<Option<T>>>,
    tmp_buf: Vec<Option<T>>,
    out_buf: Vec<Option<T>>,
}

impl<T> SigT for OptionFirstSome<T>
where
    T: Clone,
{
    type Item = Option<T>;

    fn sample(&mut self, ctx: &SigCtx) -> impl Buf<Self::Item> {
        self.tmp_buf.resize(ctx.num_samples, None);
        self.out_buf.clear();
        self.out_buf.resize(ctx.num_samples, None);
        for sig in &mut self.sigs {
            sig.sample_into_buf(ctx, &mut self.tmp_buf);
            for (out, x) in self.out_buf.iter_mut().zip(self.tmp_buf.iter()) {
                if out.is_some() {
                    continue;
                }
                *out = x.clone();
            }
        }
        &self.out_buf
    }
}

pub fn sig_option_first_some<T: Clone>(
    s: impl IntoIterator<Item = SigBoxed<Option<T>>>,
) -> Sig<impl SigT<Item = Option<T>>> {
    Sig(OptionFirstSome {
        sigs: s.into_iter().collect(),
        out_buf: Vec::new(),
        tmp_buf: Vec::new(),
    })
}

impl<T, S> Sig<S>
where
    T: Clone,
    S: SigT<Item = Option<T>>,
{
    pub fn option_or<O>(self, other: O) -> Sig<impl SigT<Item = Option<T>>>
    where
        O: SigT<Item = Option<T>>,
    {
        self.zip(other).map(|(s, o)| s.or(o))
    }
}

impl<T, S> Sig<S>
where
    T: Clone + Default,
    S: SigT<Item = Option<T>>,
{
    /// Reverses the `gated` function. Given a sequence of optional values, produces a sequence of
    /// values and a gate sequence. When the input signal is `None`, the output value sequence uses
    /// the previous `Some` value of the input, or `Default::default()` if no `Some` value has been
    /// produced yet.
    pub fn ungated(
        self,
    ) -> (Sig<impl SigT<Item = T>>, Sig<impl SigT<Item = bool>>) {
        let mut prev_value = T::default();
        self.map_mut(move |value_opt| match value_opt {
            Some(value) => {
                prev_value = value.clone();
                (value, true)
            }
            None => (prev_value.clone(), false),
        })
        .unzip()
    }
}

pub struct WithBuf<S, F>
where
    S: SigT,
    F: FnMut(&[S::Item]),
{
    sig: S,
    f: F,
    buf: Vec<S::Item>,
}

impl<S, F> SigT for WithBuf<S, F>
where
    S: SigT,
    F: FnMut(&[S::Item]),
{
    type Item = S::Item;

    fn sample(&mut self, ctx: &SigCtx) -> impl Buf<Self::Item> {
        let ret = self.sig.sample(ctx);
        ret.clone_to_vec(&mut self.buf);
        (self.f)(&self.buf);
        ret
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

pub struct Zip3<A, B, C>
where
    A: SigT,
    B: SigT,
    C: SigT,
{
    a: A,
    b: B,
    c: C,
}

impl<A, B, C> SigT for Zip3<A, B, C>
where
    A: SigT,
    B: SigT,
    C: SigT,
{
    type Item = (A::Item, B::Item, C::Item);

    fn sample(&mut self, ctx: &SigCtx) -> impl Buf<Self::Item> {
        Zip3Buf::new(self.a.sample(ctx), self.b.sample(ctx), self.c.sample(ctx))
    }
}

pub struct Zip4<A, B, C, D>
where
    A: SigT,
    B: SigT,
    C: SigT,
    D: SigT,
{
    a: A,
    b: B,
    c: C,
    d: D,
}

impl<A, B, C, D> SigT for Zip4<A, B, C, D>
where
    A: SigT,
    B: SigT,
    C: SigT,
    D: SigT,
{
    type Item = (A::Item, B::Item, C::Item, D::Item);

    fn sample(&mut self, ctx: &SigCtx) -> impl Buf<Self::Item> {
        Zip4Buf::new(
            self.a.sample(ctx),
            self.b.sample(ctx),
            self.c.sample(ctx),
            self.d.sample(ctx),
        )
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

pub struct SignedTo01<S>(S)
where
    S: SigT<Item = f32>;

impl<S> SigT for SignedTo01<S>
where
    S: SigT<Item = f32>,
{
    type Item = f32;

    fn sample(&mut self, ctx: &SigCtx) -> impl Buf<Self::Item> {
        MapBuf::new(self.0.sample(ctx), signed_to_01)
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
    shared_cached_sig: Arc<RwLock<SigCached<S>>>,
    buf: Vec<S::Item>,
}

impl<S> SigShared<S>
where
    S: SigT,
{
    fn new(sig: S) -> Self {
        SigShared {
            shared_cached_sig: Arc::new(RwLock::new(SigCached::new(sig))),
            buf: Vec::new(),
        }
    }

    /// Call a supplied function on the `SigT` implementation inside `self`.
    pub fn with_inner<T, F>(&self, mut f: F) -> T
    where
        F: FnMut(&S) -> T,
    {
        let inner_cached = self.shared_cached_sig.read().unwrap();
        f(&inner_cached.sig)
    }
}

impl<S> Clone for SigShared<S>
where
    S: SigT,
{
    fn clone(&self) -> Self {
        SigShared {
            shared_cached_sig: Arc::clone(&self.shared_cached_sig),
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
        let mut shared_cached_sig = self.shared_cached_sig.write().unwrap();
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
    Sig(SigShared::new(sig))
}

pub struct SigFn<F, T>
where
    F: FnMut(&SigCtx) -> T,
    T: Clone + Default,
{
    f: F,
    buf: Vec<T>,
}

impl<F, T> SigT for SigFn<F, T>
where
    F: FnMut(&SigCtx) -> T,
    T: Clone + Default,
{
    type Item = T;

    fn sample(&mut self, ctx: &SigCtx) -> impl Buf<Self::Item> {
        self.buf.resize_with(ctx.num_samples, Default::default);
        for out in self.buf.iter_mut() {
            *out = (self.f)(ctx);
        }
        &self.buf
    }
}

impl<F, T> Sig<SigFn<F, T>>
where
    F: FnMut(&SigCtx) -> T,
    T: Clone + Default,
{
    pub fn from_fn(f: F) -> Self {
        Self(SigFn { f, buf: Vec::new() })
    }
}

pub struct SigBufFn<F, T>
where
    F: FnMut(&SigCtx, &mut Vec<T>),
    T: Clone,
{
    f: F,
    buf: Vec<T>,
}

impl<F, T> SigT for SigBufFn<F, T>
where
    F: FnMut(&SigCtx, &mut Vec<T>),
    T: Clone,
{
    type Item = T;

    fn sample(&mut self, ctx: &SigCtx) -> impl Buf<Self::Item> {
        (self.f)(ctx, &mut self.buf);
        &self.buf
    }
}

impl<F, T> Sig<SigBufFn<F, T>>
where
    F: FnMut(&SigCtx, &mut Vec<T>),
    T: Clone,
{
    pub fn from_buf_fn(f: F) -> Self {
        Self(SigBufFn { f, buf: Vec::new() })
    }
}

#[derive(Default)]
pub struct SigVar<T>(Arc<RwLock<T>>);

impl<T> SigVar<T> {
    pub fn new(value: T) -> Self {
        Self(Arc::new(RwLock::new(value)))
    }

    pub fn set(&self, value: T) {
        *self.0.write().unwrap() = value;
    }
}

impl<T> Sig<SigVar<T>>
where
    T: Clone,
{
    pub fn set(&self, value: T) {
        self.0.set(value)
    }
}

pub fn sig_var<T: Clone>(value: T) -> Sig<SigVar<T>> {
    Sig(SigVar::new(value))
}

impl<T> Clone for SigVar<T> {
    fn clone(&self) -> Self {
        Self(Arc::clone(&self.0))
    }
}

impl<T> SigT for SigVar<T>
where
    T: Clone,
{
    type Item = T;

    fn sample(&mut self, ctx: &SigCtx) -> impl Buf<Self::Item> {
        ConstBuf {
            count: ctx.num_samples,
            value: self.0.read().unwrap().clone(),
        }
    }
}

pub struct SigBoxedVarUnshared<T: Clone> {
    sig_boxed: Arc<RwLock<SigBoxed<T>>>,
    buf: Vec<T>,
}

impl<T: Clone> Clone for SigBoxedVarUnshared<T> {
    fn clone(&self) -> Self {
        Self {
            sig_boxed: Arc::clone(&self.sig_boxed),
            buf: Vec::new(),
        }
    }
}

impl<T: Clone> SigBoxedVarUnshared<T> {
    pub fn new<S>(sig: S) -> Self
    where
        S: SigT<Item = T> + Sync + Send + 'static,
    {
        SigBoxedVarUnshared {
            sig_boxed: Arc::new(RwLock::new(sig_boxed(sig))),
            buf: Vec::new(),
        }
    }

    pub fn set<S>(&self, sig: S)
    where
        S: SigT<Item = T> + Sync + Send + 'static,
    {
        *self.sig_boxed.write().unwrap() = sig_boxed(sig);
    }
}

impl<T: Clone> Sig<SigBoxedVarUnshared<T>> {
    pub fn set<S>(&self, sig: S)
    where
        S: SigT<Item = T> + Sync + Send + 'static,
    {
        self.0.set(sig);
    }
}

impl<T> SigBoxedVarUnshared<T>
where
    T: Clone + Sync + Send + 'static,
{
    pub fn new_const(value: T) -> Self {
        SigBoxedVarUnshared {
            sig_boxed: Arc::new(RwLock::new(sig_boxed(Const(value)))),
            buf: Vec::new(),
        }
    }
}

impl<T> SigBoxedVarUnshared<T>
where
    T: Clone + Default + Sync + Send + 'static,
{
    pub fn new_default() -> Self {
        Self::new_const(T::default())
    }
}

impl<T: Clone> SigT for SigBoxedVarUnshared<T> {
    type Item = T;

    fn sample(&mut self, ctx: &SigCtx) -> impl Buf<Self::Item> {
        let mut unlocked = self.sig_boxed.write().unwrap();
        unlocked.sample_into_buf(ctx, &mut self.buf);
        &self.buf
    }
}

impl<T: Clone> SigSampleIntoBufT for SigBoxedVarUnshared<T> {
    type Item = T;

    fn sample_into_buf(&mut self, ctx: &SigCtx, buf: &mut Vec<Self::Item>) {
        self.sig_boxed.write().unwrap().sample_into_buf(ctx, buf);
    }

    fn sample_into_slice(
        &mut self,
        ctx: &SigCtx,
        stride: usize,
        offset: usize,
        out: &mut [Self::Item],
    ) {
        self.sig_boxed
            .write()
            .unwrap()
            .sample_into_slice(ctx, stride, offset, out);
    }
}

pub fn sig_boxed_var<S>(sig: S) -> Sig<SigBoxedVar<S::Item>>
where
    S: SigT + Sync + Send + 'static,
{
    Sig(SigBoxedVar::new(sig))
}

#[derive(Clone)]
pub struct SigBoxedVar<T: Clone>(Sig<SigShared<SigBoxedVarUnshared<T>>>);

impl<T: Clone> SigT for SigBoxedVar<T> {
    type Item = T;

    fn sample(&mut self, ctx: &SigCtx) -> impl Buf<Self::Item> {
        self.0.sample(ctx)
    }
}

impl<T: Clone> SigBoxedVar<T> {
    pub fn new<S>(sig: S) -> Self
    where
        S: SigT<Item = T> + Sync + Send + 'static,
    {
        Self(sig_shared(SigBoxedVarUnshared::new(sig)))
    }

    pub fn set<S>(&self, sig: S)
    where
        S: SigT<Item = T> + Sync + Send + 'static,
    {
        let sig_cached = self.0.0.shared_cached_sig.write().unwrap();
        sig_cached.sig.set(sig);
    }
}

impl<T: Clone> Sig<SigBoxedVar<T>> {
    pub fn set<S>(&self, sig: S)
    where
        S: SigT<Item = T> + Sync + Send + 'static,
    {
        self.0.set(sig);
    }
}

impl<T: Clone> SigSampleIntoBufT for SigBoxedVar<T> {
    type Item = T;

    fn sample_into_buf(&mut self, ctx: &SigCtx, buf: &mut Vec<Self::Item>) {
        self.0.sample_into_buf(ctx, buf);
    }

    fn sample_into_slice(
        &mut self,
        ctx: &SigCtx,
        stride: usize,
        offset: usize,
        out: &mut [Self::Item],
    ) {
        self.0.sample_into_slice(ctx, stride, offset, out);
    }
}

pub trait Triggerable {
    type Item: Clone;

    fn into_sig<T>(self, trig: T) -> impl SigT<Item = Self::Item>
    where
        T: SigT<Item = bool>;
}
