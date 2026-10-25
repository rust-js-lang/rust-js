// Adapters a struct holds, lent by `by_ref()` to what steps them part of the way:
// what's left is still there, from either end, as each holds its source
// and its state (ADR 0542).
use std::iter::{Filter, Skip};
use std::slice::Iter;

struct Evens<'a> {
    it: Filter<Iter<'a, u32>, fn(&&u32) -> bool>,
}

struct Rest<'a> {
    it: Skip<Iter<'a, u32>>,
}

fn even(x: &&u32) -> bool {
    *x % 2 == 0
}

fn main() {
    let v = vec![1, 2, 3, 4, 5, 6, 7, 8];
    let mut e = Evens { it: v.iter().filter(even as fn(&&u32) -> bool) };
    let first: Vec<_> = e.it.by_ref().take(2).collect();
    println!("{first:?} {:?}", e.it.next());
    let mut r = Rest { it: v.iter().skip(2) };
    for x in r.it.by_ref() {
        if *x == 4 {
            break;
        }
    }
    println!("{:?} {:?} {}", r.it.next_back(), r.it.next(), r.it.len());
    let mut again = Rest { it: v.iter().skip(1) };
    let back: Vec<_> = again.it.by_ref().rev().take(2).collect();
    println!("{back:?} {:?}", again.it.next());
}
