// A generic impl's constant of its parameters, as num-traits' `ConstZero`
// of `Wrapping<T>` is: made from its parameters' constants, each read,
// through generic code.

use std::num::Wrapping;

trait ConstZero {
    const ZERO: Self;
}

impl ConstZero for u8 {
    const ZERO: u8 = 0;
}

impl ConstZero for i64 {
    const ZERO: i64 = -0;
}

impl<T: ConstZero> ConstZero for Wrapping<T> {
    const ZERO: Self = Wrapping(T::ZERO);
}

trait Width {
    const BITS: u32;
}

impl Width for u8 {
    const BITS: u32 = 8;
}

impl Width for u32 {
    const BITS: u32 = 32;
}

struct Pair<A, B>(A, B);

impl<A: Width, B: Width> Width for Pair<A, B> {
    const BITS: u32 = A::BITS + B::BITS;
}

fn zero<T: ConstZero>() -> T {
    T::ZERO
}

fn bits<T: Width>() -> u32 {
    T::BITS
}

fn main() {
    println!("{} {}", zero::<Wrapping<u8>>().0, zero::<Wrapping<Wrapping<i64>>>().0 .0);
    println!("{} {} {}", bits::<Pair<u8, u32>>(), bits::<Pair<Pair<u8, u8>, u32>>(), bits::<u8>());
    println!("{}", <Pair<u32, u32> as Width>::BITS);
}
