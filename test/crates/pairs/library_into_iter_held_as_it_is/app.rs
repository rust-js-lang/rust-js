//! Each stepped as Rust steps it: the vectors' `IntoIter`s, their lengths
//! known, and a chain of them.
use dep::{Wrap, both, chained};

pub fn main() {
    let pair = both(vec!['a', 'b'], [1u8, 2, 3]);
    println!("{} {}", pair.a.len(), pair.b.len());
    let mut held = Wrap(vec![4, 5]).pair();
    held.t.next();
    println!("{:?}", held.t.as_slice());
    println!("{:?}", Wrap(vec![6]).first());
    let all: Vec<u32> = chained(vec![7, 8], [9]).collect();
    println!("{all:?}");
}
