// An entry's value put in where it's missing, then used, `m.entry(k).
// or_default().push(v)`, is `if (!m.has(k)) { m.set(k, []); }
// m.get(k).push(v);`, as a person writes it: where the map and the key read
// the same each time and what's put in only reads variables. One whose key
// is made where it's given, or whose value's maker does more, keeps the
// helper, which makes each once.
use std::collections::{BTreeMap, HashMap};

fn grouped(items: &[(&str, u32)]) -> Vec<(String, Vec<u32>)> {
    let mut groups: HashMap<String, Vec<u32>> = HashMap::new();
    for &(key, value) in items {
        let key = key.to_string();
        groups.entry(key.clone()).or_default().push(value);
        groups.entry(key).or_insert_with(Vec::new).push(value * 10);
    }
    let mut groups: Vec<(String, Vec<u32>)> = groups.into_iter().collect();
    groups.sort();
    groups
}

fn ordered(items: &[(u32, char)]) -> BTreeMap<u32, String> {
    let mut by: BTreeMap<u32, String> = BTreeMap::new();
    for &(key, c) in items {
        by.entry(key).or_insert(String::new()).push(c);
    }
    by
}

fn made_apart(items: &[u32]) -> (BTreeMap<String, Vec<u32>>, u32) {
    let mut made = 0;
    let mut by: BTreeMap<String, Vec<u32>> = BTreeMap::new();
    for &item in items {
        by.entry(format!("k{}", item % 2))
            .or_insert_with(|| {
                made += 1;
                Vec::new()
            })
            .push(item);
    }
    (by, made)
}

fn fresh(made: &mut u32) -> Vec<u32> {
    *made += 1;
    Vec::new()
}

fn each_made(items: &[u32]) -> (HashMap<u32, Vec<u32>>, u32) {
    let mut made = 0;
    let mut by: HashMap<u32, Vec<u32>> = HashMap::new();
    for &item in items {
        by.entry(item % 2).or_insert(fresh(&mut made)).push(item);
    }
    (by, made)
}

fn main() {
    let (by, made) = each_made(&[1, 2, 3]);
    println!("{:?} {:?} {made}", by.get(&0), by.get(&1));
    println!("{:?}", grouped(&[("a", 1), ("b", 2), ("a", 3)]));
    println!("{:?}", ordered(&[(2, 'x'), (1, 'y'), (2, 'z')]));
    println!("{:?}", made_apart(&[1, 2, 3, 4, 5]));
}
