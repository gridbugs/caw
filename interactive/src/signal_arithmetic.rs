use crate::signal::Signal;
use std::ops::{Add, Div, Mul, Sub};

macro_rules! impl_binary_op {
    ($trait:ident, $fn:ident) => {
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
    };
}

impl_binary_op!(Add, add);
impl_binary_op!(Sub, sub);
impl_binary_op!(Mul, mul);
impl_binary_op!(Div, div);
