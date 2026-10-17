// A B-tree's range of a pair of `Bound`s, `(Excluded(&2), Unbounded)`, as
// of any range: `$treeRange` takes its start's and its end's `Bound`, as
// `start_bound()` and `end_bound()` give them (ADR 0325).

use std::collections::{BTreeMap, BTreeSet};
use std::ops::Bound::{self, Excluded, Included, Unbounded};

fn between(set: &BTreeSet<u32>, bounds: (Bound<u32>, Bound<u32>)) -> Vec<u32> {
    set.range(bounds).copied().collect()
}

fn middle() -> std::ops::Range<u32> {
    2..4
}

fn main() {
    let tree: BTreeMap<u32, &str> = BTreeMap::from([(1, "a"), (2, "b"), (3, "c"), (4, "d"), (5, "e")]);
    let keys = |bounds: (Bound<&u32>, Bound<&u32>)| tree.range(bounds).map(|(k, _)| *k).collect::<Vec<_>>();
    println!("{:?}", keys((Excluded(&2), Unbounded)));
    println!("{:?}", keys((Included(&2), Excluded(&4))));
    println!("{:?}", keys((Excluded(&1), Included(&4))));
    println!("{:?}", keys((Unbounded, Included(&3))));
    println!("{:?}", keys((Excluded(&3), Excluded(&4))));
    println!("{:?}", keys((Unbounded, Unbounded)));
    println!("{:?}", tree.range((Excluded(&4), Unbounded)).next());
    println!("{:?}", tree.range(2..=3).collect::<Vec<_>>());
    let (low, high) = (3, 5);
    println!("{:?}", tree.range(low..high).count());
    println!("{:?}", tree.range(middle()).count());

    let set: BTreeSet<u32> = (1..=6).collect();
    println!("{:?}", between(&set, (Excluded(1), Excluded(4))));
    println!("{:?}", between(&set, (Included(5), Unbounded)));
    let mut values = tree.clone();
    for (_, v) in values.range_mut((Excluded(&3), Unbounded)) {
        *v = "z";
    }
    println!("{:?}", values);
}
