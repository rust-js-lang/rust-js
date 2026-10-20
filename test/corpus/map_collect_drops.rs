// A map or a set made of items with a destructor drops what an equal one
// replaces, as std does (ADR 0506, reversing ADR 0321's refusal): a hash
// map the key it's given and the value it replaces, each as it's inserted;
// a B-tree, which sorts what it's made of and keeps the last of equal ones,
// the earlier, key then value, in its keys' order, once it has them all.
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
#[derive(PartialEq, Eq, Hash, PartialOrd, Ord)]
struct Key(u8, u8);
impl Drop for Key {
    fn drop(&mut self) {
        println!("drop key {} {}", self.0, self.1);
    }
}
struct Loud(u8);
impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}
fn main() {
    let h = HashMap::from([(1, Loud(1)), (1, Loud(2))]);
    println!("h {}", h.len());
    let b = BTreeMap::from([(2, Loud(3)), (1, Loud(4)), (2, Loud(5)), (1, Loud(6))]);
    println!("b {:?}", b.values().map(|l| l.0).collect::<Vec<_>>());
    let c: BTreeMap<u8, Loud> = (0..4).map(|i| { println!("make {i}"); (i % 2, Loud(10 + i)) }).collect();
    println!("c {}", c.len());
    let s = BTreeSet::from([Key(1, 0), Key(1, 0)]);
    let hs: HashSet<Key> = [Key(2, 0), Key(2, 0)].into_iter().collect();
    let km: HashMap<Key, Loud> = [(Key(3, 0), Loud(20)), (Key(3, 0), Loud(21))].into_iter().collect();
    println!("{} {} {}", s.len(), hs.len(), km.len());
}
