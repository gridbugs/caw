use crate::{FrameSig, FrameSigT, SigCtx};
use std::{
    iter::Sum,
    ops::{Add, Div, Mul, Sub},
};

macro_rules! impl_op {
    ($frame_sig_mod:ident, $trait:ident, $fn:ident) => {
        pub mod $frame_sig_mod {
            use crate::{FrameSig, FrameSigT, SigCtx};
            use std::ops::$trait;

            /// Signal for applying the operation pairwise to each element of a pair of signals
            pub struct Op<L, R>
            where
                L: FrameSigT,
                R: FrameSigT,
                L::Item: $trait<R::Item>,
            {
                pub lhs: L,
                pub rhs: R,
            }

            impl<L, R> FrameSigT for Op<L, R>
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
                type Output = FrameSig<Op<S, R>>;

                fn $fn(self, rhs: R) -> Self::Output {
                    FrameSig(Op { lhs: self.0, rhs })
                }
            }
        }
    };
}

macro_rules! impl_arith_op {
    ($frame_sig_mod:ident, $trait:ident, $fn:ident) => {
        /// Operate on a signal and an f32 where the RHS is wrapped in the `Sig` type.
        impl<R> $trait<FrameSig<R>> for f32
        where
            R: FrameSigT<Item = f32>,
            f32: $trait<R::Item>,
        {
            type Output = FrameSig<$frame_sig_mod::Op<f32, R>>;

            fn $fn(self, rhs: FrameSig<R>) -> Self::Output {
                FrameSig($frame_sig_mod::Op {
                    lhs: self,
                    rhs: rhs.0,
                })
            }
        }
    };
}

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

impl_op!(sig_add, Add, add);
impl_op!(sig_sub, Sub, sub);
impl_op!(sig_mul, Mul, mul);
impl_op!(sig_div, Div, div);
impl_op!(sig_bit_and, BitAnd, bitand);
impl_op!(sig_bit_or, BitOr, bitor);

impl_arith_op!(sig_add, Add, add);
impl_arith_op!(sig_sub, Sub, sub);
impl_arith_op!(sig_mul, Mul, mul);
impl_arith_op!(sig_div, Div, div);
