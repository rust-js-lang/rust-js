// `B::from_iter(items)` of a map or a set is `items.into_iter().collect()`:
// its items in its order, a B-tree's its keys' (ADR 0160).
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet, VecDeque};
fn main() {
    let s = BTreeSet::from([3, 1, 2]);
    let v = Vec::from_iter(s.clone());
    let d = VecDeque::from_iter(&s);
    println!("{:?} {:?}", v, d);
    let m = BTreeMap::from([("b", 2), ("a", 1)]);
    let pairs = Vec::from_iter(m.clone());
    let back: HashMap<&str, i32> = HashMap::from_iter(m);
    let mut keys: Vec<_> = back.keys().collect();
    keys.sort();
    println!("{:?} {:?}", pairs, keys);
    let h = HashSet::from([5, 4]);
    let mut hv = Vec::from_iter(h);
    hv.sort();
    let t = BTreeSet::from_iter(hv.clone());
    println!("{:?} {:?}", hv, t);
    let strs = BTreeSet::from(["x".to_string(), "y".to_string()]);
    let joined = String::from_iter(strs);
    println!("{}", joined);
}
