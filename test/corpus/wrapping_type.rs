// `std::num::Wrapping`: a number whose arithmetic wraps, shown as the
// number, in concrete code and through generic code's operator bounds, as
// num-traits' `pow` and `Zero` take one.

use std::num::Wrapping;
use std::ops::{Add, Mul};

fn pow<T: Mul<Output = T> + Copy>(base: T, one: T, exp: u32) -> T {
    let mut acc = one;
    for _ in 0..exp {
        acc = acc * base;
    }
    acc
}

fn total<T: Add<Output = T> + Copy>(items: &[T], zero: T) -> T {
    items.iter().fold(zero, |acc, &x| acc + x)
}

fn same<T: PartialEq>(a: T, b: T) -> bool {
    a == b
}

fn main() {
    let a = Wrapping(200u8);
    let b = a * Wrapping(2) + Wrapping(100);
    println!("{} {:?} {} {}", b, b, b.0, b == Wrapping(244));
    let mut c = Wrapping(i32::MAX);
    c += Wrapping(1);
    c -= Wrapping(2);
    println!("{} {} {}", c, -Wrapping(i8::MIN), !Wrapping(0u16));
    println!("{} {} {}", Wrapping(1u32) << 35, Wrapping(-16i64) >> 2, Wrapping(1u8) << 9);
    println!("{} {}", pow(Wrapping(3u8), Wrapping(1), 7), pow(3u32, 1, 7));
    println!("{} {}", total(&[Wrapping(250u8), Wrapping(10)], Wrapping(0)), same(Wrapping(5u64), Wrapping(5)));
    println!("{:?} {}", Wrapping(7u8).max(Wrapping(9)), Wrapping(3i16) < Wrapping(-1));
}
