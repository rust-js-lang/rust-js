// `or_insert_with_key` gives its function the key, where it inserts; where
// the key is there, the function, unrun, is dropped, with what it holds.
use std::collections::{BTreeMap, HashMap};
struct Noisy(&'static str);
impl Drop for Noisy {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
struct K(u8, String);
fn main() {
    let mut m: HashMap<K, String> = HashMap::new();
    for (i, w) in ["a", "b", "a"].iter().enumerate() {
        let n = Noisy(w);
        let v = m
            .entry(K(1, w.to_string()))
            .or_insert_with_key(move |k| format!("{}-{}-{}", k.1, n.0, i));
        v.push('!');
    }
    let mut keys: Vec<_> = m.iter().collect();
    keys.sort();
    println!("{:?}", keys);
    let mut b: BTreeMap<u32, Vec<u32>> = BTreeMap::new();
    for x in [3, 1, 3, 2] {
        b.entry(x).or_insert_with_key(|k| vec![*k * 10]).push(x);
    }
    println!("{:?}", b);
}
