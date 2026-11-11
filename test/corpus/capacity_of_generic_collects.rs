// Generic code's `collect()` into a `B: FromIterator` its caller gives as a
// `Vec`, of an iterator std can't collect in place, has std's capacity
// (#1263), by the way std's `SpecFromIter` collects it (`IVecCollect`,
// #1239): a `TrustedLen`'s by its fold, room for its exact length first;
// another's by `next`, its first item, then room for its hint's lower bound
// and one more, at least four, growing by doubling where it's passed. What
// its caller gives is made knowing its hint where JS's own iterator, a set's
// `iter()`, `Array.from(set).values()`, and its own adapters would lose it:
// an enumeration's, `$enumerate`, and a chain's parts', `$taking`.
use std::collections::{BTreeSet, HashSet};

/// A crate's iterator of `n` items, whose hint is its own.
struct Own {
    left: u32,
    lower: usize,
}

impl Iterator for Own {
    type Item = u32;
    fn next(&mut self) -> Option<u32> {
        self.left = self.left.checked_sub(1)?;
        Some(self.left)
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.lower, None)
    }
}

fn collected<I: Iterator, B: FromIterator<I::Item>>(it: I) -> B {
    it.collect()
}

fn main() {
    let set: HashSet<u32> = (0..9).collect();
    let sorted: BTreeSet<u32> = (0..40).collect();
    // By its fold: `TrustedLen`.
    let mapped: Vec<u32> = collected((0..6).map(|x| x * 2));
    println!("mapped {} {}", mapped.len(), mapped.capacity());
    // By `next`: a crate's own hint, under it and over it.
    for (left, lower) in [(5, 0), (5, 2), (9, 3), (3, 7)] {
        let own: Vec<u32> = collected(Own { left, lower });
        println!("own {left} {lower}: {} {}", own.len(), own.capacity());
    }
    // By `next`: a set's, whose JS loses its hint where it's adapted.
    let enumerated: Vec<(usize, &u32)> = collected(set.iter().enumerate());
    println!("enumerated {} {}", enumerated.len(), enumerated.capacity());
    let filtered: Vec<&u32> = collected(set.iter().filter(|x| **x % 2 == 0));
    println!("filtered {} {}", filtered.len(), filtered.capacity());
    let chained: Vec<u32> = collected(sorted.iter().take(4).chain(set.iter().take(3)).copied());
    println!("chained {} {}", chained.len(), chained.capacity());
    let skipped: Vec<u32> = collected(sorted.iter().skip(30).map(|x| x + 1));
    println!("skipped {} {}", skipped.len(), skipped.capacity());
}
