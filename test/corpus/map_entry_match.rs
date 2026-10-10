// A map's `Entry` matched as `Occupied` or `Vacant`: whether its key is
// there, each arm given its entry, which reads, writes or puts in.
use std::collections::btree_map;
use std::collections::hash_map::Entry;
use std::collections::{BTreeMap, HashMap};

fn main() {
    let mut counts: HashMap<&str, i32> = HashMap::new();
    for word in ["a", "b", "a", "c", "a"] {
        match counts.entry(word) {
            Entry::Occupied(mut seen) => *seen.get_mut() += 1,
            Entry::Vacant(new) => {
                println!("first {}", new.key());
                new.insert(1);
            }
        }
    }
    let mut sorted: Vec<_> = counts.iter().collect();
    sorted.sort();
    println!("{sorted:?}");

    let mut names: BTreeMap<i32, String> = BTreeMap::new();
    for k in [2, 1, 2] {
        let entry = names.entry(k);
        if let btree_map::Entry::Occupied(o) = &entry {
            println!("{} is {}", o.key(), o.get());
        }
        match entry {
            btree_map::Entry::Occupied(o) => println!("took {}", o.remove()),
            btree_map::Entry::Vacant(v) => {
                let key = *v.key();
                v.insert(key.to_string()).push('!');
            }
        }
    }
    println!("{names:?}");

    let mut empty: HashMap<String, u8> = HashMap::new();
    if let Entry::Vacant(v) = empty.entry("z".to_string()) {
        let key = v.into_key();
        println!("{key} {}", empty.len());
    }
}
