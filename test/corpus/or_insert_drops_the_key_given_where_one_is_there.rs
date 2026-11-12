// `entry(k).or_insert(v)` of a key and a value with destructors: where an
// equal key's there, the key given is dropped, then the value given, as
// std's `or_insert` drops them, its `OccupiedEntry`'s key first; where it's
// missing, both are kept (ADRs 0321, 0467).
use std::collections::BTreeMap;

#[derive(PartialEq, Eq, PartialOrd, Ord)]
struct Key(&'static str);

impl Drop for Key {
    fn drop(&mut self) {
        println!("drop key {}", self.0);
    }
}

struct Val(u8);

impl Drop for Val {
    fn drop(&mut self) {
        println!("drop val {}", self.0);
    }
}

fn main() {
    let mut m: BTreeMap<Key, Val> = BTreeMap::new();
    m.entry(Key("a")).or_insert(Val(1));
    println!("inserted");
    let v = m.entry(Key("a")).or_insert(Val(2));
    println!("there {}", v.0);
    m.entry(Key("b")).or_insert(Val(3)).0 += 1;
    println!("len {}", m.len());
}
