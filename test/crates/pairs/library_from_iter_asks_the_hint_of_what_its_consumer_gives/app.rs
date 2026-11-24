//! A set's takes, skips and filters, and a `Vec` and a range, each given
//! a library's `from_iter`.
use std::collections::{BTreeSet, HashSet};
pub fn main() {
    let set: HashSet<u32> = (0..9).collect();
    let sorted: BTreeSet<u32> = (0..40).collect();
    let v = dep::vec_of(sorted.iter().take(4).chain(set.iter().take(3)).copied());
    println!("vec {} {}", v.len(), v.capacity());
    let d = dep::deque_of(sorted.iter().take(4).chain(set.iter().take(3)).copied());
    println!("deque {} {}", d.len(), d.capacity());
    let owned = vec![1u32, 2, 3, 4, 5];
    let v2 = dep::vec_of(owned.clone());
    println!("owned {} {}", v2.len(), v2.capacity());
    let v3 = dep::vec_of_refs(&owned);
    println!("refs {} {}", v3.len(), v3.capacity());
    let v4 = dep::vec_of(0..6);
    println!("range {} {}", v4.len(), v4.capacity());
    let v5 = dep::vec_of(set.iter().filter(|x| **x > 2).copied());
    println!("filter {} {}", v5.len(), v5.capacity());
    let v6 = dep::vec_of(set.iter().skip(3).copied());
    println!("skip {} {}", v6.len(), v6.capacity());
}
