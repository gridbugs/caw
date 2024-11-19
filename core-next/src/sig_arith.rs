use crate::{Buf, Sig, SigCtx, SigT};
use std::{
    iter::Sum,
    ops::{Add, Div, Mul, Sub},
};

macro_rules! impl_binary_op {
    ($sig_struct:ident, $trait:ident, $fn:ident) => {
        /// Signal for applying the operation pairwise to each element of a pair of signals
        pub struct $sig_struct<L, R>
        where
            L: SigT,
            R: SigT,
            L::Item: Clone,
            R::Item: Clone,
            L::Item: $trait<R::Item>,
        {
            lhs: L,
            rhs: R,
            buf: Vec<<L::Item as $trait<R::Item>>::Output>,
        }

        impl<L, R> SigT for $sig_struct<L, R>
        where
            L: SigT,
            R: SigT,
            L::Item: Clone,
            R::Item: Clone,
            L::Item: $trait<R::Item>,
        {
            type Item = <L::Item as $trait<R::Item>>::Output;

            fn sample(&mut self, ctx: &SigCtx) -> impl Buf<Self::Item> {
                let buf_lhs = self.lhs.sample(ctx);
                let buf_rhs = self.rhs.sample(ctx);
                self.buf.clear();
                self.buf.extend(
                    buf_lhs
                        .iter()
                        .cloned()
                        .zip(buf_rhs.iter().cloned())
                        .map(|(lhs, rhs)| lhs.$fn(rhs)),
                );
                &self.buf
            }
        }

        /// Operate on a pair of signals where at least the LHS is wrapped in the `Sig` type.
        impl<S, R> $trait<R> for Sig<S>
        where
            S: SigT,
            R: SigT,
            S::Item: $trait<R::Item>,
            S::Item: Clone,
            R::Item: Clone,
        {
            type Output = Sig<$sig_struct<S, R>>;

            fn $fn(self, rhs: R) -> Self::Output {
                Sig($sig_struct {
                    lhs: self.0,
                    rhs,
                    buf: Vec::new(),
                })
            }
        }

        /// Operate on a signal and an f32 where teh RHS is wrapped in the `Sig` type.
        impl<R> $trait<Sig<R>> for f32
        where
            R: SigT,
            R::Item: Clone,
            f32: $trait<R::Item>,
        {
            type Output = Sig<$sig_struct<f32, R>>;

            fn $fn(self, rhs: Sig<R>) -> Self::Output {
                Sig($sig_struct {
                    lhs: self,
                    rhs: rhs.0,
                    buf: Vec::new(),
                })
            }
        }

        /// Operate on a signal and an i32 where teh RHS is wrapped in the `Sig` type.
        impl<R> $trait<Sig<R>> for i32
        where
            R: SigT,
            R::Item: Clone,
            f32: $trait<R::Item>,
        {
            type Output = Sig<$sig_struct<i32, R>>;

            fn $fn(self, rhs: Sig<R>) -> Self::Output {
                Sig($sig_struct {
                    lhs: self,
                    rhs: rhs.0,
                    buf: Vec::new(),
                })
            }
        }
    };
}

impl_binary_op!(SigAdd, Add, add);
impl_binary_op!(SigSub, Sub, sub);
impl_binary_op!(SigMul, Mul, mul);
impl_binary_op!(SigDiv, Div, div);

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
        self.buf.resize(ctx.num_samples, 0.0);
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
        let buf = Vec::new();
        Sig(SignalSum { sigs, buf })
    }
}
