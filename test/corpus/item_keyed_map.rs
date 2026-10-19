// A trait's default that makes a map keyed by `Self::Item`, as itertools'
// `counts` does, is given the class of its caller's items, `SelfItemMap`
// (ADR 0466), and drops a key its consumer gives with a destructor as
// Rust's does (ADR 0467).

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

#[derive(Hash, PartialEq, Eq, Debug)]
struct Loud(u8);

impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

fn main() {
    let words = ["x", "y", "x"].into_iter().counts();
    println!("{:?} {:?}", words.get("x"), words.get("z"));
    let points = vec![P(0, 1), P(0, 1), P(2, 2)].into_iter().counts();
    println!("{:?} {}", points.get(&P(0, 1)), points.len());
    let louds = vec![Loud(1), Loud(1)].into_iter().counts();
    println!("{:?}", louds.get(&Loud(1)));
}
