use crate::signal::{const_, Sf64, Signal};
use std::{
    iter::Sum,
    ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Sub, SubAssign},
};

macro_rules! impl_binary_op {
    ($trait:ident, $fn:ident, $trait_assign:ident, $fn_assign:ident) => {
        // applying the operator pairwise between two signals
        impl<T> $trait for Signal<T>
        where
            T: $trait + Clone + 'static,
            <T as $trait>::Output: Clone,
        {
            type Output = Signal<<T as $trait>::Output>;

            fn $fn(self, rhs: Self) -> Self::Output {
                self.both(&rhs).map(|(lhs, rhs)| lhs.$fn(rhs))
            }
        }

        // applying the operator pairwise between two signals (lhs is a reference)
        impl<T> $trait<Signal<T>> for &Signal<T>
        where
            T: $trait + Clone + 'static,
            <T as $trait>::Output: Clone,
        {
            type Output = Signal<<T as $trait>::Output>;

            fn $fn(self, rhs: Signal<T>) -> Self::Output {
                self.both(&rhs).map(|(lhs, rhs)| lhs.$fn(rhs))
            }
        }

        // applying the operator pairwise between two signals (rhs is a reference)
        impl<T> $trait<&Signal<T>> for Signal<T>
        where
            T: $trait + Clone + 'static,
            <T as $trait>::Output: Clone,
        {
            type Output = Signal<<T as $trait>::Output>;

            fn $fn(self, rhs: &Self) -> Self::Output {
                self.both(rhs).map(|(lhs, rhs)| lhs.$fn(rhs))
            }
        }

        // applying the operator pairwise between two references to signals
        impl<T> $trait<&Signal<T>> for &Signal<T>
        where
            T: $trait + Clone + 'static,
            <T as $trait>::Output: Clone,
        {
            type Output = Signal<<T as $trait>::Output>;

            fn $fn(self, rhs: &Signal<T>) -> Self::Output {
                self.both(rhs).map(|(lhs, rhs)| lhs.$fn(rhs))
            }
        }

        // applying the operator between the signal and a scalar
        impl<T> $trait<T> for Signal<T>
        where
            T: $trait + Copy + 'static,
            <T as $trait>::Output: Clone,
        {
            type Output = Signal<<T as $trait>::Output>;
            fn $fn(self, rhs: T) -> Self::Output {
                self.map(move |lhs| lhs.$fn(rhs))
            }
        }

        // applying the operator between a reference to the signal and a scalar
        impl<T> $trait<T> for &Signal<T>
        where
            T: $trait + Copy + 'static,
            <T as $trait>::Output: Clone,
        {
            type Output = Signal<<T as $trait>::Output>;
            fn $fn(self, rhs: T) -> Self::Output {
                self.map(move |lhs| lhs.$fn(rhs))
            }
        }

        impl<T> $trait_assign for Signal<T>
        where
            T: $trait<Output = T> + Clone + 'static,
        {
            fn $fn_assign(&mut self, rhs: Self) {
                *self = (&*self).$fn(rhs);
            }
        }

        impl<T> $trait_assign<&Self> for Signal<T>
        where
            T: $trait<Output = T> + Clone + 'static,
        {
            fn $fn_assign(&mut self, rhs: &Self) {
                *self = (&*self).$fn(rhs);
            }
        }
    };
}

impl_binary_op!(Add, add, AddAssign, add_assign);
impl_binary_op!(Sub, sub, SubAssign, sub_assign);
impl_binary_op!(Mul, mul, MulAssign, mul_assign);
impl_binary_op!(Div, div, DivAssign, div_assign);

impl Sum for Sf64 {
    fn sum<I: Iterator<Item = Self>>(mut iter: I) -> Self {
        if let Some(mut total) = iter.next() {
            for signal in iter {
                total += signal;
            }
            total
        } else {
            const_(0.0)
        }
    }
}
