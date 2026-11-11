// `Entry::insert_entry(v)` and `VacantEntry::insert_entry(v)` write or put
// `v` in and give the `OccupiedEntry` of it, of the entry's one search: a
// key found by its own `Hash` is hashed once, by `entry`, however its
// entry is used after; what an occupied one held is dropped as it's
// replaced. Hash maps and B-trees alike. It was refused (#1369).
use std::cell::Cell;
use std::collections::btree_map;
use std::collections::hash_map::{self, Entry};
use std::collections::{BTreeMap, HashMap};
use std::hash::{Hash, Hasher};

thread_local! {
    static HASHED: Cell<u32> = const { Cell::new(0) };
}

/// A key whose `Hash` counts its calls.
#[derive(PartialEq, Eq, Debug, Clone, Copy)]
struct Counted(u32);

impl Hash for Counted {
    fn hash<H: Hasher>(&self, state: &mut H) {
        HASHED.set(HASHED.get() + 1);
        self.0.hash(state);
    }
}

#[derive(Debug)]
struct Loud(u32);

impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

fn hashed_since(before: u32) -> u32 {
    HASHED.get() - before
}

fn main() {
    let mut m: HashMap<Counted, Loud> = HashMap::new();
    let before = HASHED.get();
    let e = m.entry(Counted(1)).insert_entry(Loud(10));
    println!("{:?} {:?} hashed {}", e.key(), e.get(), hashed_since(before));

    let before = HASHED.get();
    let mut e = m.entry(Counted(1)).insert_entry(Loud(11));
    println!("replaced");
    e.get_mut().0 += 100;
    println!("{:?} hashed {}", e.get(), hashed_since(before));

    let before = HASHED.get();
    if let Entry::Vacant(v) = m.entry(Counted(2)) {
        let o: hash_map::OccupiedEntry<'_, Counted, Loud> = v.insert_entry(Loud(20));
        let key = *o.key();
        let removed = o.remove();
        println!("{key:?} {removed:?} hashed {}", hashed_since(before));
    }
    println!("{} {:?}", m.len(), m.get(&Counted(1)));

    let mut words: HashMap<&str, u32> = HashMap::new();
    *words.entry("a").insert_entry(1).get_mut() += 4;
    let e = words.entry("a").insert_entry(7);
    println!("{} {}", e.key(), e.get());

    let mut t: BTreeMap<u8, Loud> = BTreeMap::new();
    let o = t.entry(3).insert_entry(Loud(30));
    println!("{:?}", o.get());
    let o = t.entry(3).insert_entry(Loud(31));
    println!("{:?} {:?}", o.key(), o.get());
    if let btree_map::Entry::Vacant(v) = t.entry(1) {
        let o = v.insert_entry(Loud(1));
        println!("{:?}", o.remove_entry());
    }
    println!("{:?}", t.keys().collect::<Vec<_>>());
}
