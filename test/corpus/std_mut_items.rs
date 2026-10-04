// A `&mut` std hands out to a number or a string in a collection, used as a
// value: given to a closure, kept, or looped over where an index loop
// doesn't reach, as a map's values are. Each writes the item it points at.

use std::collections::{BTreeMap, HashMap};

fn bump(n: &mut i32) {
    *n += 100;
}

fn peek(o: Option<&mut i32>) -> i32 {
    o.map_or(0, |x| *x)
}

fn main() {
    // Given to closures.
    let mut scores = vec![1, 2, 3];
    scores.iter_mut().for_each(|s| *s *= 10);
    scores.iter_mut().filter(|s| **s > 10).for_each(|s| *s += 1);
    for (i, s) in scores.iter_mut().enumerate() {
        *s += i as i32;
    }
    println!("{:?}", scores);
    let mut names = vec![String::from("ann"), String::from("bo")];
    names.iter_mut().for_each(|n| n.push('!'));
    println!("{:?}", names);

    // A map's values, and its entries.
    let mut stock: HashMap<&str, i32> = HashMap::from([("apples", 3), ("pears", 0)]);
    for n in stock.values_mut() {
        *n += 1;
    }
    for (name, n) in stock.iter_mut() {
        if name.starts_with('a') {
            *n *= 2;
        }
    }
    let mut sorted: Vec<(&str, i32)> = stock.iter().map(|(k, v)| (*k, *v)).collect();
    sorted.sort();
    println!("{:?}", sorted);
    let mut ordered: BTreeMap<u8, f64> = BTreeMap::from([(2, 0.5), (1, 1.5)]);
    ordered.values_mut().for_each(|x| *x *= 2.0);
    for (_, x) in &mut ordered {
        *x += 0.25;
    }
    // In the keys' order, as a B-tree's are.
    let mut visited = Vec::new();
    for (k, x) in ordered.iter_mut() {
        visited.push(*k);
        *x -= 0.5;
    }
    let halves: Vec<f64> = ordered.values_mut().map(|x| *x / 2.0).collect();
    println!("{:?} {:?} {:?}", ordered, visited, halves);

    // Kept in a variable, and passed on.
    let mut counts: HashMap<char, i32> = HashMap::new();
    counts.insert('a', 1);
    let a = counts.get_mut(&'a');
    if let Some(n) = a {
        *n += 1;
        bump(n);
    }
    let mut row = vec![5, 6];
    let last = row.last_mut().unwrap();
    *last = 60;
    bump(row.first_mut().unwrap());
    if let Some(x) = row.get_mut(1) {
        bump(x);
    }
    println!("{:?} {:?}", counts, row);

    // Passed on in an `Option`, matched, and collected.
    let seen = peek(counts.get_mut(&'a')) + peek(counts.get_mut(&'z'));
    let r = counts.get_mut(&'a');
    let found = match r {
        Some(x) => {
            *x -= 2;
            *x
        }
        None => 0,
    };
    let mut cells = vec![1, 2, 3];
    let refs: Vec<&mut i32> = cells.iter_mut().filter(|c| **c != 2).collect();
    for r in refs {
        *r = -*r;
    }
    println!("{} {} {:?}", seen, found, cells);
}
