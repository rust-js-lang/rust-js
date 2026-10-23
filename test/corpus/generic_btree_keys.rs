// A `BTreeMap` or a `BTreeSet` of a type parameter's keys, `K: Ord`: its
// class is its caller's, as a `HashMap`'s is, and its order `K`'s `Ord`.
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Debug;

fn distinct<T: Ord>(xs: Vec<T>) -> usize {
    xs.into_iter().collect::<BTreeSet<T>>().len()
}

fn counts<K: Ord + Clone + Debug>(xs: &[K]) -> Vec<(K, u32)> {
    let mut seen = BTreeMap::new();
    for x in xs {
        *seen.entry(x.clone()).or_insert(0) += 1;
    }
    seen.into_iter().collect()
}

fn main() {
    println!("{} {}", distinct(vec![3, 1, 3, 2]), distinct(vec!["b", "a", "b"]));
    println!("{:?}", counts(&["x", "y", "x"]));
    println!("{:?}", counts(&[(2, 'b'), (1, 'a'), (2, 'b')]));
}
