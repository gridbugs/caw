use crate::{Buf, Sig, SigCtx, SigT};
use std::{
    iter::Sum,
    ops::{Add, Mul},
};

// Add

pub struct SignalAdd<L, R>
where
    L: SigT,
    R: SigT,
    L::Item: Clone,
    R::Item: Clone,
    L::Item: Add<R::Item>,
{
    lhs: L,
    rhs: R,
    buf: Vec<<L::Item as Add<R::Item>>::Output>,
}

impl<L, R> SigT for SignalAdd<L, R>
where
    L: SigT,
    R: SigT,
    L::Item: Clone,
    R::Item: Clone,
    L::Item: Add<R::Item>,
{
    type Item = <L::Item as Add<R::Item>>::Output;

    fn sample(&mut self, ctx: &SigCtx) -> impl Buf<Self::Item> {
        let buf_lhs = self.lhs.sample(ctx);
        let buf_rhs = self.rhs.sample(ctx);
        self.buf.clear();
        self.buf.extend(
            buf_lhs
                .iter()
                .cloned()
                .zip(buf_rhs.iter().cloned())
                .map(|(lhs, rhs)| lhs + rhs),
        );
        &self.buf
    }
}

impl<S, R> Add<R> for Sig<S>
where
    S: SigT,
    R: SigT,
    S::Item: Add<R::Item>,
    S::Item: Clone,
    R::Item: Clone,
{
    type Output = Sig<SignalAdd<S, R>>;

    fn add(self, rhs: R) -> Self::Output {
        Sig(SignalAdd {
            lhs: self.0,
            rhs,
            buf: Vec::new(),
        })
    }
}

// Sum

pub struct SignalSum<S>
where
    S: SigT,
    S::Item: Add,
{
    sigs: Vec<Sig<S>>,
    buf: Vec<f32>,
}

impl<S> SigT for SignalSum<S>
where
    S: SigT<Item = f32>,
{
    type Item = f32;

    fn sample(&mut self, ctx: &SigCtx) -> impl Buf<Self::Item> {
        self.buf.fill(0.0);
        for buffered_signal in &mut self.sigs {
            let buf = buffered_signal.sample(ctx);
            for (out, sample) in self.buf.iter_mut().zip(buf.iter()) {
                *out += sample;
            }
        }
        &self.buf
    }
}

impl<S> Sum<Sig<S>> for Sig<SignalSum<S>>
where
    S: SigT<Item = f32>,
{
    fn sum<I: Iterator<Item = Sig<S>>>(iter: I) -> Self {
        let sigs = iter.collect::<Vec<_>>();
        let buf = vec![0.0; sigs.len()];
        Sig(SignalSum { sigs, buf })
    }
}

// Mul

pub struct SignalMul<L, R>
where
    L: SigT,
    R: SigT,
    L::Item: Clone,
    R::Item: Clone,
    L::Item: Mul<R::Item>,
{
    lhs: L,
    rhs: R,
    buf: Vec<<L::Item as Mul<R::Item>>::Output>,
}

impl<L, R> SigT for SignalMul<L, R>
where
    L: SigT,
    R: SigT,
    L::Item: Clone,
    R::Item: Clone,
    L::Item: Mul<R::Item>,
{
    type Item = <L::Item as Mul<R::Item>>::Output;

    fn sample(&mut self, ctx: &SigCtx) -> impl Buf<Self::Item> {
        let buf_lhs = self.lhs.sample(ctx);
        let buf_rhs = self.rhs.sample(ctx);
        self.buf.clear();
        self.buf.extend(
            buf_lhs
                .iter()
                .cloned()
                .zip(buf_rhs.iter().cloned())
                .map(|(lhs, rhs)| lhs * rhs),
        );
        &self.buf
    }
}

impl<S, R> Mul<R> for Sig<S>
where
    S: SigT,
    R: SigT,
    S::Item: Mul<R::Item>,
    S::Item: Clone,
    R::Item: Clone,
{
    type Output = Sig<SignalMul<S, R>>;

    fn mul(self, rhs: R) -> Self::Output {
        Sig(SignalMul {
            lhs: self.0,
            rhs,
            buf: Vec::new(),
        })
    }
}
