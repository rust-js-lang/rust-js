// A map's entry held before it's filled, `$entry`'s, of a key with a
// destructor: the key it's given dropped where an equal's there, as
// `entry` drops it, and not again by `or_insert`; a key that's new kept
// (ADR 0467).

use std::collections::HashMap;

#[derive(Debug, PartialEq, Eq, Hash)]
struct Key(u32);

impl Drop for Key {
    fn drop(&mut self) {
        println!("drop key {}", self.0);
    }
}

fn main() {
    let mut m = HashMap::new();
    m.insert(Key(1), 10);
    let there = m.entry(Key(1));
    println!("held");
    *there.or_insert(0) += 1;
    let new = m.entry(Key(2));
    println!("held new");
    *new.or_insert(5) += 1;
    println!("{:?} {:?}", m.get(&Key(1)), m.get(&Key(2)));
    m.remove(&Key(2));
    println!("end");
}
