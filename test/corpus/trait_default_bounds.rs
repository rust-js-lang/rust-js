// A trait's default method comparing `Self`, as num-traits' `Signed` and
// `One` do: `<` through a `PartialOrd` supertrait, of a number in the impl
// it's copied into, and `==` by the method's own `where Self: PartialEq`, of
// a generic impl's type.

use std::num::Wrapping;
use std::ops::Mul;

trait Zero {
    fn zero() -> Self;
}

trait Signed: Zero + PartialOrd + Copy {
    fn negative(self) -> bool {
        self < Self::zero()
    }
    fn at_most_zero(self) -> bool {
        self <= Self::zero()
    }
}

impl Zero for f64 {
    fn zero() -> f64 {
        0.0
    }
}
impl Signed for f64 {}

impl Zero for i32 {
    fn zero() -> i32 {
        0
    }
}
impl Signed for i32 {}

trait One: Sized + Mul<Self, Output = Self> {
    fn one() -> Self;
    fn is_one(&self) -> bool
    where
        Self: PartialEq,
    {
        *self == Self::one()
    }
}

impl One for u8 {
    fn one() -> u8 {
        1
    }
}

impl<T: One> One for Wrapping<T>
where
    Wrapping<T>: Mul<Output = Wrapping<T>>,
{
    fn one() -> Self {
        Wrapping(T::one())
    }
}

fn main() {
    println!("{} {} {} {}", (-1.5f64).negative(), 2.0f64.negative(), f64::NAN.negative(), 0i32.at_most_zero());
    println!("{} {} {}", Wrapping(1u8).is_one(), Wrapping(3u8).is_one(), 1u8.is_one());
}
