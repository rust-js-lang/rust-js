// A map's or a set's keys with a destructor drop as Rust's do (ADR 0467):
// the key `insert` and `entry` are given where an equal's there, which it
// keeps, the one `remove` takes out, and those `retain` doesn't keep; each
// key, then its value, as it's cleared or dropped, a B-tree's in its keys'
// order. A `HashMap`'s order is Rust's hash order, so each here is dropped
// with one entry left.

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

#[derive(Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
struct Key(u32);

#[derive(Debug, PartialEq, Eq, Hash)]
struct Id(u8);

impl Drop for Key {
    fn drop(&mut self) {
        println!("drop key {}", self.0);
    }
}

fn main() {
    let mut m = HashMap::new();
    m.insert(Key(1), 10);
    println!("insert {:?}", m.insert(Key(1), 11));
    *m.entry(Key(1)).or_insert(0) += 1;
    *m.entry(Key(2)).or_insert(0) += 1;
    *m.entry(Key(2)).or_insert_with(|| 5) += 1;
    println!("remove {:?}", m.remove(&Key(2)));
    println!("get {:?}", m.get(&Key(1)));
    println!("removed {:?}", m.remove_entry(&Key(1)).map(|(k, v)| (k.0, v)));
    m.extend([(Key(5), 1), (Key(5), 2)]);
    m.insert(Key(6), 6);
    m.retain(|k, _| k.0 != 6);
    println!("--");
    drop(m);
    let mut s = HashSet::new();
    println!("add {}", s.insert(Key(3)));
    println!("add {}", s.insert(Key(3)));
    println!("del {}", s.remove(&Key(3)));
    s.insert(Key(4));
    println!("replace {:?}", s.replace(Key(4)).map(|k| k.0));
    println!("take {:?}", s.take(&Key(4)).map(|k| k.0));
    s.insert(Key(7));
    s.retain(|k| k.0 != 7);
    s.insert(Key(8));
    drop(s);
    let mut t = BTreeMap::new();
    t.insert(Key(9), "a");
    t.insert(Key(8), "b");
    t.insert(Key(9), "c");
    t.clear();
    let mut sorted = BTreeSet::new();
    sorted.insert(Key(11));
    sorted.insert(Key(10));
    println!("sorted add {}", sorted.insert(Key(11)));
    drop(sorted);
    // The key a map holds, given back, not the equal one it's asked with.
    let asked = Id(1);
    let mut ids = HashMap::new();
    ids.insert(Id(1), "x");
    let (key, _) = ids.remove_entry(&asked).expect("there");
    println!("removed is asked {}", std::ptr::eq(&key, &asked));
    println!("end");
}
