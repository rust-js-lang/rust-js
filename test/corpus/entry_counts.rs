// A map's entry, its value put in where its key is missing: a count added
// to through its `&mut`, a list pushed to, and one made by a closure, then
// filled already.
use std::collections::HashMap;

fn main() {
    let mut counts: HashMap<char, u32> = HashMap::new();
    for c in "banana".chars() {
        *counts.entry(c).or_insert(0) += 1;
    }
    let mut groups: HashMap<bool, Vec<u32>> = HashMap::new();
    for n in 1..6 {
        groups.entry(n % 2 == 0).or_default().push(n);
    }
    let mut lens: HashMap<&str, usize> = HashMap::new();
    *lens.entry("abc").or_insert_with(|| 3) *= 2;
    *lens.entry("abc").or_insert_with(|| 7) += 1;
    let mut c: Vec<(char, u32)> = counts.into_iter().collect();
    c.sort();
    let mut g: Vec<(bool, Vec<u32>)> = groups.into_iter().collect();
    g.sort();
    let mut l: Vec<(&str, usize)> = lens.into_iter().collect();
    l.sort();
    println!("{c:?} {g:?} {l:?}");
}
