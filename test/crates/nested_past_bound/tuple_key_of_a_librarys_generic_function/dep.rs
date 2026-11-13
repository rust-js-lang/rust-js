// map_keyed_by_a_tuple_of_a_type_parameter's items with `Default` and
// `Copy` derived, a library: its generic functions relay a tuple key a
// level deeper each round, to how deep a type may nest (#1708).
use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::hash::{Hash, Hasher};
#[derive(Debug, Clone, Copy)]
pub struct Digit(pub u32);
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
pub struct Backwards(pub u32);
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
pub trait Tools: Iterator + Sized {
    fn counts(self) -> HashMap<Self::Item, usize>
    where
        Self::Item: Hash + Eq,
    {
        let mut counts = HashMap::new();
        for item in self {
            *counts.entry(item).or_insert(0) += 1;
        }
        counts
    }
    fn positions_seen(self) -> usize
    where
        Self::Item: Hash + Eq,
    {
        self.enumerate().counts().len()
    }
    fn ordered(self) -> BTreeMap<(usize, Self::Item), ()>
    where
        Self::Item: Ord,
    {
        self.enumerate().map(|pair| (pair, ())).collect()
    }
}
impl<I: Iterator> Tools for I {
    fn counts(self) -> HashMap<Self::Item, usize>
    where
        Self::Item: Hash + Eq,
    {
        let mut counts = HashMap::new();
        for item in self {
            *counts.entry(item).or_insert(0) += 1;
        }
        counts
    }
    fn positions_seen(self) -> usize
    where
        Self::Item: Hash + Eq,
    {
        self.enumerate().counts().len()
    }
    fn ordered(self) -> BTreeMap<(usize, Self::Item), ()>
    where
        Self::Item: Ord,
    {
        self.enumerate().map(|pair| (pair, ())).collect()
    }
}
pub fn distinct<T: Hash + Eq + Copy>(items: &[T]) -> usize {
    items.iter().map(|&x| (x, 0u8)).collect::<HashSet<_>>().len()
}
pub fn pairs<T: Hash + Eq + Copy>(items: &[T]) -> usize {
    items
        .windows(2)
        .map(|w| [w[0], w[1]])
        .collect::<HashSet<[T; 2]>>()
        .len()
}
pub fn sorted<T: Ord + Copy + std::fmt::Debug>(items: &[T]) -> Vec<(T, usize)> {
    let set: BTreeSet<(T, usize)> = items.iter().copied().zip(0..).collect();
    set.into_iter().collect()
}
