// Maps and sets keyed by a reference to a type parameter's value,
// `HashSet<&T>`, `HashMap<&T, usize>`, `BTreeSet<&T>`: a `&T` is its `T`,
// its class the one its caller gives for `T`, so tuples are found by value,
// as TheAlgorithms/rust's `have_same_elements` compares two sorts' items.
use std::collections::{BTreeSet, HashMap, HashSet};
use std::hash::Hash;

fn counts<T: Eq + Hash>(items: &[T]) -> HashMap<&T, usize> {
    let mut counts = HashMap::new();
    for item in items {
        *counts.entry(item).or_insert(0) += 1;
    }
    counts
}

fn most<T: Eq + Hash + Ord>(items: &[T]) -> Option<(&T, usize)> {
    let counts = counts(items);
    let mut best: Vec<(&T, usize)> = counts.into_iter().collect();
    best.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(b.0)));
    best.first().copied()
}

fn distinct_sorted<T: Ord>(items: &[T]) -> Vec<&T> {
    let set: BTreeSet<&T> = items.iter().collect();
    set.into_iter().collect()
}

fn has<T: Eq + Hash>(items: &[T], x: &T) -> bool {
    let set: HashSet<&T> = items.iter().collect();
    set.contains(x)
}

fn same<T: Eq + Hash>(a: &[T], b: &[T]) -> bool {
    let set_a: HashSet<&T> = a.iter().collect();
    let set_b: HashSet<&T> = b.iter().collect();
    set_a == set_b
}

fn main() {
    let pairs = [(1, 'a'), (2, 'b'), (1, 'a'), (3, 'c'), (2, 'b'), (1, 'a')];
    println!("{:?}", most(&pairs));
    println!("{:?}", distinct_sorted(&pairs));
    println!("{} {}", has(&pairs, &(3, 'c')), has(&pairs, &(4, 'd')));
    println!(
        "{} {}",
        same(&pairs, &[(3, 'c'), (2, 'b'), (1, 'a')]),
        same(&pairs, &[(1, 'a')])
    );
    let words = ["b".to_string(), "a".to_string(), "b".to_string()];
    println!("{:?} {:?}", most(&words), distinct_sorted(&words));
}
