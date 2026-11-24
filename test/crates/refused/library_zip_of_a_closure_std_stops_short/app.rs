//! A consumer giving the library's zip a `map` of a slice's iterator
//! whose closure runs code: refused at its call, as one crate's own
//! generic code is, where rust-js's zip would call the closure once more
//! than std's (#2542).
use dep::{paired, zipped};

pub fn main() {
    let a = [1u8, 2, 3, 4];
    let b = [10u8, 20];
    let mut calls = Vec::new();
    let z = zipped(
        a.iter().map(|x| {
            calls.push(*x);
            x * 2
        }),
        b.iter(),
    );
    println!("{z:?} {calls:?}");
    let mut seen = 0;
    let p = paired(
        a.iter().map(|x| {
            seen += 1;
            *x
        }),
        b.iter(),
    );
    println!("{p:?} {seen}");
}
