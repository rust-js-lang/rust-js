// `B::from_iter(items)`, which `items.collect()` is, of each kind of
// collection, an array, an iterator and a generic one, as itertools writes it.

use std::collections::{BTreeSet, HashMap, VecDeque};

fn sorted<I: Iterator<Item = u32>>(it: I) -> Vec<u32> {
    let mut v = Vec::from_iter(it);
    v.sort();
    v
}

fn main() {
    println!("{:?}", sorted([3, 1, 2].into_iter()));
    let v = Vec::from_iter([5, 4].into_iter().map(|x| x * 2));
    let s = String::from_iter(['a', 'b']);
    let backwards = String::from_iter("abc".chars().rev());
    let set = BTreeSet::from_iter([3, 1, 3]);
    let m: HashMap<&str, u32> = HashMap::from_iter([("a", 1)]);
    let d = VecDeque::from_iter(1..4);
    println!("{v:?} {s} {backwards} {set:?} {m:?} {d:?}");
}
