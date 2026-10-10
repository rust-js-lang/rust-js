// A range collected into a set or a map is its items, as into a `Vec`.
use std::collections::{BTreeSet, HashMap, HashSet};

fn main() {
    let s: HashSet<i32> = (1..=6).collect();
    let t: BTreeSet<i32> = (1..4).collect();
    let m: HashMap<i32, i32> = (1..=3).map(|i| (i, i * i)).collect();
    let u: BTreeSet<char> = ('a'..='c').collect();
    let mut more: HashSet<i32> = HashSet::new();
    more.extend(2..5);
    println!("{} {:?} {} {:?} {}", s.len(), t, m[&3], u, more.len());
}
