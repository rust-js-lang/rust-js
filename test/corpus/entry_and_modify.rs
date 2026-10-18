// A map entry's `and_modify(f)`: `f` given the value where the key is
// there, a handle on a number or text, and the entry for `or_insert` and
// the rest after it, of a `HashMap`'s and a `BTreeMap`'s (ADR 0345).
use std::collections::{BTreeMap, HashMap};
fn main() {
    let mut inventory: HashMap<&str, (u32, f64)> = HashMap::new();
    inventory.insert("apple", (3, 0.5));
    inventory.entry("apple").and_modify(|e| e.0 += 2).or_insert((0, 0.0));
    inventory.entry("pear").and_modify(|e| e.0 += 2).or_insert((1, 0.25));
    let mut counts: BTreeMap<char, u32> = BTreeMap::new();
    for c in "banana".chars() {
        counts.entry(c).and_modify(|n| *n += 1).or_insert(1);
    }
    let mut names: HashMap<u8, String> = HashMap::new();
    names.insert(1, "a".into());
    names.entry(1).and_modify(|s| s.push('!')).or_default();
    names.entry(2).and_modify(|s| s.push('?')).or_insert_with(|| "new".into());
    let mut keys: Vec<_> = inventory.iter().collect();
    keys.sort_by(|a, b| a.0.cmp(b.0));
    let mut ns: Vec<_> = names.into_iter().collect();
    ns.sort();
    println!("{keys:?} {counts:?} {ns:?}");
}
