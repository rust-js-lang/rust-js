// A `BTreeMap`'s `first_entry` and `last_entry`: its least or greatest
// key's `OccupiedEntry`, which reads, writes and takes out what's there.
use std::collections::BTreeMap;

fn main() {
    let mut names: BTreeMap<i32, String> = (1..=4).map(|i| (i, i.to_string())).collect();
    if let Some(mut first) = names.first_entry() {
        first.get_mut().push('!');
        println!("{} {}", first.key(), first.get());
    }
    if let Some(last) = names.last_entry() {
        println!("{:?}", last.remove_entry());
    }
    println!("{names:?}");

    let mut counts: BTreeMap<&str, i32> = BTreeMap::from([("b", 2), ("a", 1), ("c", 3)]);
    if let Some(mut first) = counts.first_entry() {
        *first.get_mut() += 10;
        let old = first.insert(100);
        println!("{old} {}", first.get());
    }
    *counts.last_entry().unwrap().into_mut() *= 7;
    println!("{counts:?}");
    println!("{:?}", counts.first_entry().map(|e| e.remove()));
    println!("{counts:?}");
    let mut empty: BTreeMap<i32, i32> = BTreeMap::new();
    println!("{} {}", empty.first_entry().is_none(), empty.last_entry().is_none());
}
