// A B-tree given where an `IntoIterator` is taken, to `extend`, `chain`,
// `zip` or `flatten`, is stepped in its keys' order, which its JS map isn't.
use std::collections::{BTreeMap, BTreeSet};

fn main() {
    let set = BTreeSet::from([3, 1, 2]);
    let map = BTreeMap::from([(2, 'b'), (1, 'a')]);

    let mut v = vec![0];
    v.extend(set.clone());
    let mut pairs: Vec<(i32, char)> = Vec::new();
    pairs.extend(map.clone());
    let mut text = String::new();
    text.extend(BTreeSet::from(['c', 'a']));
    println!("{v:?} {pairs:?} {text}");

    let chained: Vec<i32> = [9].into_iter().chain(set.clone()).collect();
    let zipped: Vec<(i32, (i32, char))> = set.clone().into_iter().zip(map).collect();
    let flat: Vec<i32> = vec![set, BTreeSet::from([0])].into_iter().flatten().collect();
    println!("{chained:?} {zipped:?} {flat:?}");
}
