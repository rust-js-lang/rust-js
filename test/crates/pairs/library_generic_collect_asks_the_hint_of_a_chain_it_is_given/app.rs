//! Chains of takes of a set's items, whose JS iterator knows no hint, given
//! to a library's generic code that asks it, directly and through the
//! crate's own generic function.
use std::collections::{BTreeSet, HashSet};

fn via<I: Iterator<Item = u32>>(it: I) -> Vec<u32> {
    dep::collected(it.take(3).chain(std::iter::empty()))
}

pub fn main() {
    let set: HashSet<u32> = (0..9).collect();
    let sorted: BTreeSet<u32> = (0..40).collect();
    let chained: Vec<u32> = dep::collected(sorted.iter().take(4).chain(set.iter().take(3)).copied());
    println!("chained {} {}", chained.len(), chained.capacity());
    let two: Vec<u32> = dep::collected(set.iter().take(3).chain(set.iter().take(2)).copied());
    println!("two {} {}", two.len(), two.capacity());
    let within: Vec<u32> = dep::collected_within(sorted.iter().take(4).chain(set.iter().take(3)).copied());
    println!("within {} {}", within.len(), within.capacity());
    let through = via(sorted.iter().take(4).chain(set.iter().take(3)).copied());
    println!("through {} {}", through.len(), through.capacity());
    println!("{:?}", dep::hint(&sorted.iter().take(4).chain(set.iter().take(3))));
}
