// A map's or a set's owned iteration of values with a destructor (ADR
// 0059): each entry the loop's, dropped as it's done with, and what it
// doesn't reach dropped where it stops, as Rust's `IntoIter` drops it.
use std::collections::{BTreeMap, BTreeSet};

#[derive(PartialEq, Eq, PartialOrd, Ord)]
struct Loud(u32);

impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

fn first_over(map: BTreeMap<u8, Loud>, limit: u32) -> Option<u8> {
    for (key, value) in map {
        if value.0 > limit {
            return Some(key);
        }
    }
    None
}

fn main() {
    let mut map = BTreeMap::new();
    for n in 1..=4 {
        map.insert(n as u8, Loud(n * 10));
    }
    println!("{:?}", first_over(map, 15));
    let set: BTreeSet<Loud> = [Loud(1), Loud(2), Loud(3)].into_iter().collect();
    let found = set.into_iter().find(|l| l.0 == 2).map(|l| l.0);
    println!("{found:?}");
    let mut all = BTreeMap::new();
    all.insert(1u8, Loud(7));
    all.insert(2u8, Loud(8));
    println!("{}", all.into_iter().count());
}
