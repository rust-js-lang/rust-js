// What a `HashMap`'s or a `HashSet`'s `with_capacity` is given runs, for
// what it does, as a `Vec`'s does, though a JS `Map` has no capacity
// (ADR 0315, #195).
use std::cell::Cell;
use std::collections::{HashMap, HashSet};

fn room(asked: &Cell<usize>, n: usize) -> usize {
    asked.set(asked.get() + 1);
    println!("room {n}");
    n
}

fn main() {
    let asked = Cell::new(0);
    let mut scores: HashMap<&str, u32> = HashMap::with_capacity(room(&asked, 4));
    scores.insert("a", 1);
    let seen: HashSet<u8> = HashSet::with_capacity(room(&asked, 2));
    let plain: HashMap<u8, u8> = HashMap::with_capacity(8);
    println!("{} {} {} {}", scores.len(), seen.len(), plain.len(), asked.get());
}
