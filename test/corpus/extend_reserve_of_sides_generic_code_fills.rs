// `unzip()` into sides generic code is given reserves each its source's
// lower bound by its own `extend_reserve`, a `Vec`'s and a `VecDeque`'s
// `reserve`, so their capacities are std's: of a `TrustedLen`'s, whose
// `SpecExtend` writes each by `extend_one_unchecked`, as of a `filter`'s,
// whose default writes each by `extend_one` (#2395, #2414).
use std::collections::{HashSet, VecDeque};

fn split<A: Default + Extend<u32>, B: Default + Extend<u32>>(pairs: Vec<(u32, u32)>) -> (A, B) {
    pairs.into_iter().unzip()
}

fn split_filtered<A: Default + Extend<u32>, B: Default + Extend<u32>>(pairs: Vec<(u32, u32)>) -> (A, B) {
    pairs.into_iter().filter(|_| true).unzip()
}

fn endless<A: Default + Extend<u32>>() -> (A, Vec<u32>) {
    (0u32..).map(|n| (n, n)).unzip()
}

fn main() {
    let pairs: Vec<(u32, u32)> = (0..10).map(|n| (n, n * 2)).collect();
    let (a, b): (Vec<u32>, VecDeque<u32>) = split(pairs.clone());
    println!("{} {} {} {}", a.len(), a.capacity(), b.len(), b.capacity());
    let (a, b): (Vec<u32>, Vec<u32>) = split_filtered(pairs.clone());
    println!("{} {} {} {}", a.len(), a.capacity(), b.len(), b.capacity());
    let (s, b): (HashSet<u32>, Vec<u32>) = split(pairs);
    println!("{} {}", s.len(), b.capacity());
    let r = std::panic::catch_unwind(|| endless::<HashSet<u32>>().1.len());
    println!("{}", r.is_err());
}
