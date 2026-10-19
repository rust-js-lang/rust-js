// A map or a set keyed by a type parameter, or by an associated type of
// one, `Self::Item`, is of the class its caller's type says, `Map` or
// `$KeyMap`, given as `KMap` (ADR 0466): in a generic function, a trait's
// generic method and its default, as itertools' `counts` and `into_group_map`.

use std::collections::{HashMap, HashSet};
use std::hash::Hash;

fn group<I: Iterator<Item = (K, V)>, K: Hash + Eq, V>(iter: I) -> HashMap<K, Vec<V>> {
    let mut map = HashMap::new();
    for (k, v) in iter {
        map.entry(k).or_insert_with(Vec::new).push(v);
    }
    map
}

fn distinct<T: Hash + Eq + Clone>(items: &[T]) -> HashSet<T> {
    items.iter().cloned().collect()
}

trait Counting: Iterator {
    fn counts_by<K: Hash + Eq, F: FnMut(Self::Item) -> K>(self, f: F) -> HashMap<K, usize>
    where
        Self: Sized,
    {
        let mut counts = HashMap::new();
        self.map(f).for_each(|k| *counts.entry(k).or_insert(0) += 1);
        counts
    }

    fn counts(self) -> HashMap<Self::Item, usize>
    where
        Self: Sized,
        Self::Item: Hash + Eq,
    {
        self.counts_by(|x| x)
    }

    fn tally_by<K: Hash + Eq, F: FnMut(Self::Item) -> K>(self, f: F) -> HashMap<K, usize>
    where
        Self: Sized,
    {
        self.map(f).counts()
    }
}

impl<T: Iterator> Counting for T {}

#[derive(Hash, PartialEq, Eq, Debug, Clone)]
struct P(u8, u8);

fn main() {
    let g = group(vec![("a", 1), ("b", 2), ("a", 3)].into_iter());
    println!("{:?} {:?}", g.get("a"), g.get("b"));
    let pg = group(vec![(P(1, 2), 1), (P(1, 2), 2)].into_iter());
    println!("{:?}", pg.get(&P(1, 2)));
    let d = distinct(&[P(1, 1), P(1, 1), P(2, 2)]);
    println!("{} {}", d.len(), d.contains(&P(2, 2)));
    let c = ["x", "y", "x"].into_iter().counts();
    println!("{:?}", c.get("x"));
    let cp = vec![P(0, 1), P(0, 1)].into_iter().counts_by(|p| p.0);
    println!("{:?}", cp.get(&0));
    let cs = vec![P(0, 1), P(0, 1)].into_iter().counts();
    println!("{:?}", cs.get(&P(0, 1)));
    let by = ["ab", "cd", "a"].into_iter().tally_by(str::len);
    println!("{:?} {:?}", by.get(&2), by.get(&1));
}
