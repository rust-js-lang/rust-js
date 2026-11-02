//! `a * b` of the library's `Digits`, whose `mul` its JS reaches only
//! through the dictionary it exports: called through it (ADR 0914, amended).
use dep::Digits;

pub fn main() {
    let a = Digits(3);
    let b = Digits(4);
    println!("{:?} {:?} {:?}", a * b, dep::squared(a), dep::squared(Digits(70_000)));
}
