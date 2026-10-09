// Maps' and sets' own methods, as std's are (ADR 0325): retain, drain,
// set algebra, and a B-tree's ends, ranges, splits and appends in its keys'
// order.

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

fn sorted<T: Ord + Clone>(items: impl Iterator<Item = T>) -> Vec<T> {
    let mut items: Vec<T> = items.collect();
    items.sort();
    items
}

fn main() {
    let mut scores: HashMap<&str, i32> = HashMap::with_capacity(4);
    scores.reserve(8);
    scores.extend([("a", 1), ("b", 2), ("c", 3)]);
    scores.shrink_to_fit();
    scores.retain(|_, v| {
        *v += 10;
        *v % 2 == 1
    });
    println!("{:?}", sorted(scores.iter().map(|(k, v)| (*k, *v))));
    println!("{:?}", scores.get_key_value("a"));
    println!("{:?}", scores.remove_entry("c"));
    let drained = sorted(scores.drain());
    println!("{drained:?} {}", scores.is_empty());
    let mut names = HashMap::from([(1, "one".to_string()), (2, "two".to_string())]);
    println!("{:?} {:?}", sorted(names.clone().into_keys()), sorted(names.clone().into_values()));
    names.clear();
    println!("{}", names.len());

    let a: HashSet<i32> = [1, 2, 3].into_iter().collect();
    let b: HashSet<i32> = [2, 3, 4].into_iter().collect();
    println!("{:?} {:?}", sorted(a.union(&b).copied()), sorted(a.intersection(&b).copied()));
    println!("{:?} {:?}", sorted(a.difference(&b).copied()), sorted(a.symmetric_difference(&b).copied()));
    println!("{} {} {}", a.is_subset(&b), [2].iter().copied().collect::<HashSet<_>>().is_subset(&a), a.is_disjoint(&b));
    println!("{} {:?} {:?}", b.is_superset(&[4].into_iter().collect()), a.get(&2), a.get(&9));
    let mut taken = a.clone();
    println!("{:?} {:?} {:?}", taken.take(&1), taken.replace(2), taken.replace(7));
    taken.retain(|x| x % 2 == 1);
    println!("{:?}", sorted(taken.into_iter()));

    let mut tree = BTreeMap::from([(5, "e"), (1, "a"), (3, "c"), (9, "i")]);
    println!("{:?} {:?}", tree.first_key_value(), tree.last_key_value());
    println!("{:?}", tree.range(2..5).collect::<Vec<_>>());
    println!("{:?} {:?}", tree.range(..=3).collect::<Vec<_>>(), tree.range(4..).collect::<Vec<_>>());
    println!("{:?} {:?}", tree.pop_first(), tree.pop_last());
    let mut more = BTreeMap::from([(4, "d"), (5, "E")]);
    tree.append(&mut more);
    println!("{tree:?} {}", more.len());
    let high = tree.split_off(&5);
    println!("{tree:?} {high:?}");
    tree.retain(|k, _| *k != 4);
    println!("{:?} {:?}", tree, tree.clone().into_values().collect::<Vec<_>>());

    let mut set = BTreeSet::from([4, 2, 8, 6]);
    println!("{:?} {:?} {:?}", set.first(), set.last(), set.range(3..7).collect::<Vec<_>>());
    println!("{:?} {:?} {set:?}", set.pop_first(), set.pop_last());
    let other = BTreeSet::from([1, 4, 9]);
    println!("{:?} {:?}", set.union(&other).collect::<Vec<_>>(), set.symmetric_difference(&other).collect::<Vec<_>>());
    let upper = set.split_off(&5);
    println!("{set:?} {upper:?}");
}
