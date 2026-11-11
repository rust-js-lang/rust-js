//@ library-differs
// A map or a set keyed by an array of a type parameter in generic code,
// `[T; N]`: its class is its item's class, given by the caller once,
// applied to each item (`$arrayKeyClassOf`), however long the array:
// `[T; 4096]` was one class an item, and `[T; 536870907]` took the
// compiler's memory (#1371). Keys found by value, by their own `Hash` and
// `Eq`, of the last digit alone, in a B-tree by std's order and by an `Ord`
// of their own; arrays of tuples, and of none.
use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::hash::{Hash, Hasher};

#[derive(Debug, Clone, Copy)]
struct Digit(u32);

impl Hash for Digit {
    fn hash<H: Hasher>(&self, state: &mut H) {
        (self.0 % 10).hash(state);
    }
}

impl PartialEq for Digit {
    fn eq(&self, other: &Self) -> bool {
        self.0 % 10 == other.0 % 10
    }
}

impl Eq for Digit {}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct Backwards(u32);

impl Ord for Backwards {
    fn cmp(&self, other: &Self) -> Ordering {
        other.0.cmp(&self.0)
    }
}

impl PartialOrd for Backwards {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

fn triples<T: Hash + Eq + Copy>(items: &[T]) -> usize {
    items
        .windows(3)
        .map(|w| [w[0], w[1], w[2]])
        .collect::<HashSet<[T; 3]>>()
        .len()
}

fn long_keys<T: Hash + Eq + Copy>(items: &[T]) -> HashMap<[T; 4096], usize> {
    let mut seen = HashMap::new();
    for (i, &item) in items.iter().enumerate() {
        let mut key = [items[0]; 4096];
        key[4095] = item;
        seen.entry(key).or_insert(i);
    }
    seen
}

fn ordered<T: Ord + Copy>(items: &[T]) -> Vec<[T; 2]> {
    let set: BTreeSet<[T; 2]> = items.windows(2).map(|w| [w[0], w[1]]).collect();
    set.into_iter().collect()
}

fn counted<T: Ord + Copy>(items: &[T]) -> BTreeMap<[(T, bool); 2], usize> {
    let mut counts = BTreeMap::new();
    for w in items.windows(2) {
        *counts.entry([(w[0], true), (w[1], false)]).or_insert(0) += 1;
    }
    counts
}

fn none_of<T: Hash + Eq>(items: Vec<T>) -> usize {
    items.into_iter().map(|_| [] as [T; 0]).collect::<HashSet<_>>().len()
}

fn main() {
    println!("{}", triples(&[1u8, 2, 3, 1, 2, 3, 1]));
    println!("{}", triples(&[Digit(1), Digit(2), Digit(13), Digit(21), Digit(12)]));
    let long = long_keys(&[Digit(7), Digit(17), Digit(8), Digit(27)]);
    let mut found: Vec<usize> = long.values().copied().collect();
    found.sort();
    println!("{found:?}");
    println!("{}", long_keys(&["a", "b", "a"]).len());
    println!("{:?}", ordered(&[3u32, 1, 2, 3, 1]));
    println!("{:?}", ordered(&[Backwards(3), Backwards(1), Backwards(2), Backwards(3)]));
    println!("{:?}", counted(&[Backwards(2), Backwards(1), Backwards(2), Backwards(1)]));
    println!("{}", none_of(vec![Digit(1), Digit(2)]));
    println!("{}", none_of(Vec::<String>::new()));
}
