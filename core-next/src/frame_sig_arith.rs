use crate::{sig_arith, FrameSig, FrameSigT, Sig, SigCtx, SigT};
use std::{
    iter::Sum,
    ops::{Add, Div, Mul, Sub},
};

macro_rules! impl_binary_op {
    ($frame_sig_struct:ident, $trait:ident, $fn:ident) => {
        /// Signal for applying the operation pairwise to each element of a pair of signals
        pub struct $frame_sig_struct<L, R>
        where
            L: FrameSigT,
            R: FrameSigT,
            L::Item: $trait<R::Item>,
        {
            lhs: L,
            rhs: R,
        }

        impl<L, R> FrameSigT for $frame_sig_struct<L, R>
        where
            L: FrameSigT,
            R: FrameSigT,
            L::Item: $trait<R::Item>,
            <L::Item as $trait<R::Item>>::Output: Clone,
        {
            type Item = <L::Item as $trait<R::Item>>::Output;

            fn frame_sample(&mut self, ctx: &SigCtx) -> Self::Item {
                self.lhs.frame_sample(ctx).$fn(self.rhs.frame_sample(ctx))
            }
        }

        /// Operate on a pair of signals where at least the LHS is wrapped in the `FrameSig` type.
        impl<S, R> $trait<R> for FrameSig<S>
        where
            S: FrameSigT,
            R: FrameSigT,
            S::Item: $trait<R::Item>,
            <S::Item as $trait<R::Item>>::Output: Clone,
        {
            type Output = FrameSig<$frame_sig_struct<S, R>>;

            fn $fn(self, rhs: R) -> Self::Output {
                FrameSig($frame_sig_struct { lhs: self.0, rhs })
            }
        }

        /// Add a FrameSig (lhs) to a Sig (rhs)
        impl<S, R> $trait<Sig<R>> for FrameSig<S>
        where
            S: FrameSigT,
            R: SigT,
            S::Item: $trait<R::Item>,
            <S::Item as $trait<R::Item>>::Output: Clone,
        {
            type Output = Sig<sig_arith::$frame_sig_struct<Self, R>>;

            fn $fn(self, rhs: Sig<R>) -> Self::Output {
                Sig(sig_arith::$frame_sig_struct::new(self, rhs.0))
            }
        }

        /// Operate on a signal and an f32 where teh RHS is wrapped in the `Sig` type.
        impl<R> $trait<FrameSig<R>> for f32
        where
            R: FrameSigT<Item = f32>,
            f32: $trait<R::Item>,
        {
            type Output = FrameSig<$frame_sig_struct<f32, R>>;

            fn $fn(self, rhs: FrameSig<R>) -> Self::Output {
                FrameSig($frame_sig_struct {
                    lhs: self,
                    rhs: rhs.0,
                })
            }
        }

        /// Operate on a signal and an i32 where teh RHS is wrapped in the `Sig` type.
        impl<R> $trait<FrameSig<R>> for i32
        where
            R: FrameSigT<Item = f32>,
            f32: $trait<R::Item>,
        {
            type Output = FrameSig<$frame_sig_struct<i32, R>>;

            fn $fn(self, rhs: FrameSig<R>) -> Self::Output {
                FrameSig($frame_sig_struct {
                    lhs: self,
                    rhs: rhs.0,
                })
            }
        }
    };
}

impl_binary_op!(SigAdd, Add, add);
impl_binary_op!(SigSub, Sub, sub);
impl_binary_op!(SigMul, Mul, mul);
impl_binary_op!(SigDiv, Div, div);

pub struct FrameSigSum<S>
where
    S: FrameSigT,
    S::Item: Add,
{
    sigs: Vec<FrameSig<S>>,
}

impl<S> FrameSigT for FrameSigSum<S>
where
    S: FrameSigT<Item = f32>,
{
    type Item = f32;

    fn frame_sample(&mut self, ctx: &SigCtx) -> Self::Item {
        self.sigs.iter_mut().map(|s| s.frame_sample(ctx)).sum()
    }
}

impl<S> Sum<FrameSig<S>> for FrameSig<FrameSigSum<S>>
where
    S: FrameSigT<Item = f32>,
{
    fn sum<I: Iterator<Item = FrameSig<S>>>(iter: I) -> Self {
        let sigs = iter.collect::<Vec<_>>();
        FrameSig(FrameSigSum { sigs })
    }
}
