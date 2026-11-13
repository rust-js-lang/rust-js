//@ library-differs
// map_keyed_by_a_tuple_of_a_type_parameter as the metamorphic net's
// `default-copy` rewrite makes it, `Default` and `Copy` derived where it
// derives neither: a key of a tuple of a type parameter a caller makes of
// its own key, one tuple wider each round of the relay, compiled once its
// tuples count toward how deep a relayed type may nest (#1708).
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
trait Tools: Iterator + Sized {
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
fn distinct<T: Hash + Eq + Copy>(items: &[T]) -> usize {
    items
        .iter()
        .map(|&x| (x, 0u8))
        .collect::<HashSet<_>>()
        .len()
}
fn pairs<T: Hash + Eq + Copy>(items: &[T]) -> usize {
    items
        .windows(2)
        .map(|w| [w[0], w[1]])
        .collect::<HashSet<[T; 2]>>()
        .len()
}
fn sorted<T: Ord + Copy + std::fmt::Debug>(items: &[T]) -> Vec<(T, usize)> {
    let set: BTreeSet<(T, usize)> = items.iter().copied().zip(0..).collect();
    set.into_iter().collect()
}
fn main() {
    println!("{}", vec!['a', 'b', 'a'].into_iter().positions_seen());
    println!(
        "{}",
        vec![Digit(1), Digit(11), Digit(2)]
            .into_iter()
            .positions_seen()
    );
    let ordered = vec![3u32, 1, 2].into_iter().ordered();
    println!("{:?}", ordered.keys().collect::<Vec<_>>());
    let backwards = vec![Backwards(1), Backwards(3), Backwards(2)]
        .into_iter()
        .ordered();
    println!("{:?}", backwards.keys().collect::<Vec<_>>());
    println!("{}", distinct(&[1u32, 2, 1]));
    println!("{}", distinct(&[Digit(4), Digit(14), Digit(5)]));
    println!("{}", pairs(&["a", "b", "a", "b"]));
    println!("{:?}", sorted(&[Backwards(5), Backwards(7), Backwards(5)]));
    println!("{:?}", sorted(&["b", "a"]));
}
