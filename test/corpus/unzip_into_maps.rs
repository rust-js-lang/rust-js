// `unzip` into a map or a set: made empty and extended by its side's items,
// as std's `unzip` does, a set keeping the first of equal ones.
use std::collections::{BTreeMap, BTreeSet, HashSet};

fn main() {
    let (a, b): (Vec<i32>, HashSet<i32>) = [(1, 2), (3, 2)].into_iter().unzip();
    println!("{:?} {}", a, b.len());
    let (c, d): (BTreeSet<(u8, u8)>, Vec<String>) = vec![
        ((2, 1), "x".to_string()),
        ((1, 9), "y".to_string()),
        ((2, 1), "z".to_string()),
    ]
    .into_iter()
    .unzip();
    println!("{:?} {:?}", c, d);
    let (e, f): (Vec<u8>, BTreeMap<u8, &str>) = [(1, (5, "p")), (2, (5, "q")), (3, (4, "r"))].into_iter().unzip();
    println!("{:?} {:?}", e, f);
}
