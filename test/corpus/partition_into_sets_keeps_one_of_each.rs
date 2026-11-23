// `partition()` into sets, a `HashSet`'s or a `BTreeSet`'s sides, keeps one
// of each item, as std fills each by `Default` then `extend`, which keeps the
// first and drops a duplicate, where its sides were arrays of every item
// (#2377); and `unzip()` into one beside a `Vec`, which did so already. A set
// of what has a destructor is refused (#2378).

use std::collections::{BTreeSet, HashSet};

fn sorted(set: &HashSet<u32>) -> Vec<u32> {
    let mut items: Vec<u32> = set.iter().copied().collect();
    items.sort();
    items
}

fn main() {
    let (big, small): (HashSet<u32>, HashSet<u32>) = vec![1u32, 2, 3, 3, 1].into_iter().partition(|&n| n > 1);
    println!("{:?} {:?} {} {}", sorted(&big), sorted(&small), big.len(), small.len());
    let (a, b): (BTreeSet<u32>, BTreeSet<u32>) = vec![4u32, 4, 5, 3, 3].into_iter().partition(|&n| n > 3);
    println!("{a:?} {b:?} {} {}", a.len(), b.len());
    let (firsts, seconds): (Vec<u32>, BTreeSet<u32>) = vec![(1u32, 9u32), (2, 9), (3, 8)].into_iter().unzip();
    println!("{firsts:?} {seconds:?} {}", seconds.len());
}
