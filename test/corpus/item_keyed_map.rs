//@ library-refused: a map's key with a destructor
// A trait's default that makes a map keyed by `Self::Item`, as itertools'
// `counts` does, is given the class of its caller's items, `SelfItemMap`
// (ADR 0466). As a library's, a consumer may key it by a value with a
// destructor, which a map doesn't drop yet.

use std::collections::HashMap;
use std::hash::Hash;

trait Counting: Iterator {
    fn counts(self) -> HashMap<Self::Item, usize>
    where
        Self: Sized,
        Self::Item: Hash + Eq,
    {
        let mut counts = HashMap::new();
        self.for_each(|item| *counts.entry(item).or_insert(0) += 1);
        counts
    }
}

impl<T: Iterator> Counting for T {}

#[derive(Hash, PartialEq, Eq, Debug)]
struct P(u8, u8);

fn main() {
    let words = ["x", "y", "x"].into_iter().counts();
    println!("{:?} {:?}", words.get("x"), words.get("z"));
    let points = vec![P(0, 1), P(0, 1), P(2, 2)].into_iter().counts();
    println!("{:?} {}", points.get(&P(0, 1)), points.len());
}
