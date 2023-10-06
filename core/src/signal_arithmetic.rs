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

        // implement assignment trait
        impl<T> $trait_assign for Signal<T>
        where
            T: $trait<Output = T> + Clone + 'static,
        {
            fn $fn_assign(&mut self, rhs: Self) {
                *self = (&*self).$fn(rhs);
            }
        }

        // implement assignment trait for references
        impl<T> $trait_assign<&Self> for Signal<T>
        where
            T: $trait<Output = T> + Clone + 'static,
        {
            fn $fn_assign(&mut self, rhs: &Self) {
                *self = (&*self).$fn(rhs);
            }
        }

        // implement the operation between signals and scalars with the scalar on the LHS for f64
        impl $trait<Sf64> for f64 {
            type Output = Sf64;
            fn $fn(self, rhs: Sf64) -> Self::Output {
                const_(self).$fn(rhs)
            }
        }

        // implement the operation between references to signals and scalars with the scalar on the
        // LHS for f64
        impl $trait<&Sf64> for f64 {
            type Output = Sf64;
            fn $fn(self, rhs: &Sf64) -> Self::Output {
                const_(self).$fn(rhs)
            }
        }

        // silently coerce i64 scalars into f64 when operating with Sf64
        impl $trait<i64> for Sf64 {
            type Output = Sf64;
            fn $fn(self, rhs: i64) -> Self::Output {
                self.$fn(rhs as f64)
            }
        }
        impl $trait<Sf64> for i64 {
            type Output = Sf64;
            fn $fn(self, rhs: Sf64) -> Self::Output {
                const_(self as f64).$fn(rhs)
            }
        }

        // silently coerce i64 scalars into f64 when operating with Sf64 refs
        impl $trait<i64> for &Sf64 {
            type Output = Sf64;
            fn $fn(self, rhs: i64) -> Self::Output {
                self.$fn(rhs as f64)
            }
        }
        impl $trait<&Sf64> for i64 {
            type Output = Sf64;
            fn $fn(self, rhs: &Sf64) -> Self::Output {
                const_(self as f64).$fn(rhs)
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

#[test]
fn test() {
    // Test that code involving silent scalar coersion and scalars on the RHS of operators
    // can be compiled.
    let _ = const_(0.0) + 5;
    let _ = 5 + const_(0.0);
    let _ = const_(0.0) + 5.0;
    let _ = 5.0 + const_(0.0);
}
