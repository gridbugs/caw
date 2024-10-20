use crate::{Sig, SigBuf, SigCtx};
use std::{iter::Sum, ops::Add};

pub struct SignalAdd<L, R>
where
    L: Sig,
    R: Sig,
    L::Item: Clone,
    R::Item: Clone,
    L::Item: Add<R::Item>,
{
    lhs: SigBuf<L>,
    rhs: SigBuf<R>,
}

impl<L, R> Sig for SignalAdd<L, R>
where
    L: Sig,
    R: Sig,
    L::Item: Clone,
    R::Item: Clone,
    L::Item: Add<R::Item>,
{
    type Item = <L::Item as Add<R::Item>>::Output;
    type Buf = Vec<Self::Item>;

    fn sample_batch(
        &mut self,
        ctx: &SigCtx,
        n: usize,
        sample_buffer: &mut Self::Buf,
    ) {
        self.lhs.sample_batch(ctx, n);
        self.rhs.sample_batch(ctx, n);
        for (lhs, rhs) in self.lhs.samples().zip(self.rhs.samples()) {
            sample_buffer.push(lhs.clone().add(rhs.clone()))
        }
    }
}

impl<S, R> Add<SigBuf<R>> for SigBuf<S>
where
    S: Sig,
    R: Sig,
    S::Item: Add<R::Item>,
    S::Item: Clone,
    R::Item: Clone,
{
    type Output = SigBuf<SignalAdd<S, R>>;

    fn add(self, rhs: SigBuf<R>) -> Self::Output {
        SignalAdd { lhs: self, rhs }.buffered()
    }
}

pub struct SignalSum<S>(Vec<SigBuf<S>>)
where
    S: Sig,
    S::Item: Add;

impl<S> Sig for SignalSum<S>
where
    S: Sig<Item = f32>,
{
    type Item = f32;
    type Buf = Vec<Self::Item>;

    fn sample_batch(
        &mut self,
        ctx: &SigCtx,
        n: usize,
        sample_buffer: &mut Self::Buf,
    ) {
        for _ in 0..n {
            sample_buffer.push(0.0);
        }
        for buffered_signal in &mut self.0 {
            buffered_signal.sample_batch(ctx, n);
            for (out, sample) in
                sample_buffer.iter_mut().zip(buffered_signal.samples())
            {
                *out += sample;
            }
        }
    }
}
impl<S> Sum<SigBuf<S>> for SigBuf<SignalSum<S>>
where
    S: Sig<Item = f32>,
{
    fn sum<I: Iterator<Item = SigBuf<S>>>(iter: I) -> Self {
        SignalSum(iter.collect()).buffered()
    }
}
