//@ ignore-rust-js: THIR's lowering refuses an entry's `&mut` kept in a variable, and an entry kept before it's filled
// An entry's value kept as a `&mut` in a variable, and an entry kept in a
// variable before it's filled: each writes the map.
use std::collections::HashMap;

fn main() {
    let mut lens: HashMap<&str, usize> = HashMap::new();
    let len = lens.entry("abc").or_insert_with(|| 3);
    *len *= 2;
    let entry = lens.entry("de");
    *entry.or_insert(1) += 10;
    let mut l: Vec<(&str, usize)> = lens.into_iter().collect();
    l.sort();
    println!("{l:?}");
}
