// A map of values with destructors: `retain` drops each it doesn't keep
// before the next is asked, `extend` each value it replaces, `clear` each
// in the map's order, a B-tree's its keys', and `entry(k).or_insert(v)`
// the value it's given where the key's there, as Rust's do (ADR 0321).
use std::collections::{BTreeMap, HashMap};

struct Noisy(u32);

impl Drop for Noisy {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

fn main() {
    let mut tree: BTreeMap<u32, Noisy> = (1..=6).map(|k| (k, Noisy(k * 10))).collect();
    tree.retain(|k, v| {
        println!("keep {k}? {}", v.0);
        k % 2 == 0
    });
    println!("{:?}", tree.keys().collect::<Vec<_>>());
    tree.extend(vec![(2, Noisy(21)), (7, Noisy(70))]);
    println!("{:?}", tree.values().map(|v| v.0).collect::<Vec<_>>());
    let got = tree.entry(4).or_insert(Noisy(41));
    println!("got {}", got.0);
    let new = tree.entry(8).or_insert(Noisy(80));
    println!("new {}", new.0);
    tree.clear();
    println!("cleared {}", tree.len());

    let mut hash: HashMap<&str, Noisy> = HashMap::new();
    hash.insert("a", Noisy(1));
    hash.entry("a").or_insert(Noisy(2));
    hash.extend([("a", Noisy(3))]);
    hash.retain(|_, v| v.0 > 5);
    println!("hash {}", hash.len());
    hash.insert("b", Noisy(4));
    hash.clear();
    println!("end");
}
