use crate::{Buf, FrameSig, FrameSigT, Sig, SigCtx, SigT};
use std::{
    iter::Sum,
    ops::{Add, BitAnd, BitOr, Div, Mul, Sub},
};

macro_rules! impl_op {
    ($sig_mod:ident, $trait:ident, $fn:ident) => {
        pub mod $sig_mod {
            use crate::{
                sig::{MapBuf, MapBuf2},
                Buf, FrameSigT, Sig, SigCtx, SigT,
            };
            use std::ops::$trait;

            /// Signal for applying the operation pairwise to each element of a pair of signals
            pub struct OpSigSig<L, R>
            where
                L: SigT,
                R: SigT,
                L::Item: $trait<R::Item>,
            {
                lhs: L,
                rhs: R,
            }

            impl<L, R> OpSigSig<L, R>
            where
                L: SigT,
                R: SigT,
                L::Item: $trait<R::Item>,
            {
                pub fn new(lhs: L, rhs: R) -> Self {
                    Self { lhs, rhs }
                }
            }

            impl<L, R> SigT for OpSigSig<L, R>
            where
                L: SigT,
                R: SigT,
                L::Item: $trait<R::Item>,
                <L::Item as $trait<R::Item>>::Output: Clone,
            {
                type Item = <L::Item as $trait<R::Item>>::Output;

                fn sample(&mut self, ctx: &SigCtx) -> impl Buf<Self::Item> {
                    MapBuf2::new(
                        self.lhs.sample(ctx),
                        self.rhs.sample(ctx),
                        |lhs, rhs| lhs.$fn(rhs),
                    )
                }
            }

            /// Operate on a pair of `Sig`s
            impl<S, R> $trait<Sig<R>> for Sig<S>
            where
                S: SigT,
                R: SigT,
                S::Item: $trait<R::Item>,
                <S::Item as $trait<R::Item>>::Output: Clone,
            {
                type Output = Sig<OpSigSig<S, R>>;

                fn $fn(self, rhs: Sig<R>) -> Self::Output {
                    Sig(OpSigSig::new(self.0, rhs.0))
                }
            }

            /// Signal for applying the operation pairwise to each element of a signal and a scalar
            /// where the scalar is on the rhs
            pub struct OpSigScalar<L, R>
            where
                L: SigT,
                L::Item: $trait<R>,
                <L::Item as $trait<R>>::Output: Clone,
                R: Clone,
            {
                pub lhs: L,
                pub rhs: R,
            }

            impl<L, R> SigT for OpSigScalar<L, R>
            where
                L: SigT,
                L::Item: $trait<R>,
                <L::Item as $trait<R>>::Output: Clone,
                R: Clone,
            {
                type Item = <L::Item as $trait<R>>::Output;

                fn sample(&mut self, ctx: &SigCtx) -> impl Buf<Self::Item> {
                    MapBuf::new(self.lhs.sample(ctx), |lhs| {
                        lhs.$fn(self.rhs.clone())
                    })
                }
            }

            /// Signal for applying the operation pairwise to each element of a signal and a scalar
            /// where the scalar is on the lhs
            pub struct OpScalarSig<L, R>
            where
                L: Clone,
                R: SigT,
                L: $trait<R::Item>,
                <L as $trait<R::Item>>::Output: Clone,
            {
                pub lhs: L,
                pub rhs: R,
            }

            impl<L, R> SigT for OpScalarSig<L, R>
            where
                L: Clone,
                R: SigT,
                L: $trait<R::Item>,
                <L as $trait<R::Item>>::Output: Clone,
            {
                type Item = <L as $trait<R::Item>>::Output;

                fn sample(&mut self, ctx: &SigCtx) -> impl Buf<Self::Item> {
                    MapBuf::new(self.rhs.sample(ctx), |rhs| {
                        self.lhs.clone().$fn(rhs)
                    })
                }
            }

            /// Signal for applying the operation pairwise to each element of a signal and a frame
            /// signal where the frame signal is on the rhs
            pub struct OpSigFrameSig<L, R>
            where
                L: SigT,
                R: FrameSigT,
                L::Item: $trait<R::Item>,
                <L::Item as $trait<R::Item>>::Output: Clone,
            {
                pub lhs: L,
                pub rhs: R,
            }

            impl<L, R> SigT for OpSigFrameSig<L, R>
            where
                L: SigT,
                R: FrameSigT<Item: Clone>,
                L::Item: $trait<R::Item>,
                <L::Item as $trait<R::Item>>::Output: Clone,
            {
                type Item = <L::Item as $trait<R::Item>>::Output;

                fn sample(&mut self, ctx: &SigCtx) -> impl Buf<Self::Item> {
                    let rhs = self.rhs.frame_sample(ctx);
                    MapBuf::new(self.lhs.sample(ctx), move |lhs| {
                        lhs.$fn(rhs.clone())
                    })
                }
            }

            /// Signal for applying the operation pairwise to each element of a signal and a frame
            /// signal where the frame signal is on the lhs
            pub struct OpFrameSigSig<L, R>
            where
                L: FrameSigT,
                R: SigT,
                L::Item: $trait<R::Item>,
                <L::Item as $trait<R::Item>>::Output: Clone,
            {
                pub lhs: L,
                pub rhs: R,
            }

            impl<L, R> SigT for OpFrameSigSig<L, R>
            where
                L: FrameSigT,
                R: SigT<Item: Clone>,
                L::Item: $trait<R::Item>,
                <L::Item as $trait<R::Item>>::Output: Clone,
            {
                type Item = <L::Item as $trait<R::Item>>::Output;

                fn sample(&mut self, ctx: &SigCtx) -> impl Buf<Self::Item> {
                    let lhs = self.lhs.frame_sample(ctx);
                    MapBuf::new(self.rhs.sample(ctx), move |rhs| {
                        lhs.clone().$fn(rhs)
                    })
                }
            }
        }
    };
}

macro_rules! impl_arith_op {
    ($sig_mod:ident, $trait:ident, $fn:ident) => {
        /// Operate on a `Sig` and an f32
        impl<S> $trait<f32> for Sig<S>
        where
            S: SigT,
            S::Item: $trait<f32>,
            <S::Item as $trait<f32>>::Output: Clone,
        {
            type Output = Sig<$sig_mod::OpSigScalar<S, f32>>;

            fn $fn(self, rhs: f32) -> Self::Output {
                Sig($sig_mod::OpSigScalar { lhs: self.0, rhs })
            }
        }

        /// Operate on an f32 and a `Sig`
        impl<S> $trait<Sig<S>> for f32
        where
            S: SigT<Item = f32>,
            f32: $trait<S::Item>,
        {
            type Output = Sig<$sig_mod::OpScalarSig<f32, S>>;

            fn $fn(self, rhs: Sig<S>) -> Self::Output {
                Sig($sig_mod::OpScalarSig {
                    lhs: self,
                    rhs: rhs.0,
                })
            }
        }

        /// Operate on a `Sig` and an f32
        impl<S> $trait<i32> for Sig<S>
        where
            S: SigT,
            S::Item: $trait<f32>,
            <S::Item as $trait<f32>>::Output: Clone,
        {
            type Output = Sig<$sig_mod::OpSigScalar<S, f32>>;

            fn $fn(self, rhs: i32) -> Self::Output {
                Sig($sig_mod::OpSigScalar {
                    lhs: self.0,
                    rhs: rhs as f32,
                })
            }
        }

        /// Operate on an f32 and a `Sig`
        impl<S> $trait<Sig<S>> for i32
        where
            S: SigT<Item = f32>,
            f32: $trait<S::Item>,
        {
            type Output = Sig<$sig_mod::OpScalarSig<f32, S>>;

            fn $fn(self, rhs: Sig<S>) -> Self::Output {
                Sig($sig_mod::OpScalarSig {
                    lhs: self as f32,
                    rhs: rhs.0,
                })
            }
        }

        impl<L, R> $trait<Sig<R>> for FrameSig<L>
        where
            L: FrameSigT<Item = f32>,
            R: SigT<Item = f32>,
        {
            type Output = Sig<$sig_mod::OpFrameSigSig<L, R>>;

            fn $fn(self, rhs: Sig<R>) -> Self::Output {
                Sig($sig_mod::OpFrameSigSig {
                    lhs: self.0,
                    rhs: rhs.0,
                })
            }
        }

        impl<L, R> $trait<FrameSig<R>> for Sig<L>
        where
            L: SigT<Item = f32>,
            R: FrameSigT<Item = f32>,
        {
            type Output = Sig<$sig_mod::OpSigFrameSig<L, R>>;

            fn $fn(self, rhs: FrameSig<R>) -> Self::Output {
                Sig($sig_mod::OpSigFrameSig {
                    lhs: self.0,
                    rhs: rhs.0,
                })
            }
        }
    };
}

pub struct SigSum<S>
where
    S: SigT,
    S::Item: Add,
{
    sigs: Vec<Sig<S>>,
    buf: Vec<f32>,
}

impl<S> SigT for SigSum<S>
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

impl<S> Sum<Sig<S>> for Sig<SigSum<S>>
where
    S: SigT<Item = f32>,
{
    fn sum<I: Iterator<Item = Sig<S>>>(iter: I) -> Self {
        let sigs = iter.collect::<Vec<_>>();
        let buf = Vec::new();
        Sig(SigSum { sigs, buf })
    }
}

macro_rules! impl_bool_op {
    ($sig_mod:ident, $trait:ident, $fn:ident) => {
        /// Operate on a `Sig` and an bool
        impl<S> $trait<bool> for Sig<S>
        where
            S: SigT,
            S::Item: $trait<bool>,
            <S::Item as $trait<bool>>::Output: Clone,
        {
            type Output = Sig<$sig_mod::OpSigScalar<S, bool>>;

            fn $fn(self, rhs: bool) -> Self::Output {
                Sig($sig_mod::OpSigScalar { lhs: self.0, rhs })
            }
        }

        /// Operate on an bool and a `Sig`
        impl<S> $trait<Sig<S>> for bool
        where
            S: SigT<Item = bool>,
            bool: $trait<S::Item>,
        {
            type Output = Sig<$sig_mod::OpScalarSig<bool, S>>;

            fn $fn(self, rhs: Sig<S>) -> Self::Output {
                Sig($sig_mod::OpScalarSig {
                    lhs: self,
                    rhs: rhs.0,
                })
            }
        }
    };
}

impl_op!(sig_add, Add, add);
impl_op!(sig_sub, Sub, sub);
impl_op!(sig_mul, Mul, mul);
impl_op!(sig_div, Div, div);

impl_arith_op!(sig_add, Add, add);
impl_arith_op!(sig_sub, Sub, sub);
impl_arith_op!(sig_mul, Mul, mul);
impl_arith_op!(sig_div, Div, div);

impl_op!(sig_bit_and, BitAnd, bitand);
impl_op!(sig_bit_or, BitOr, bitor);

impl_bool_op!(sig_bit_and, BitAnd, bitand);
impl_bool_op!(sig_bit_or, BitOr, bitor);
