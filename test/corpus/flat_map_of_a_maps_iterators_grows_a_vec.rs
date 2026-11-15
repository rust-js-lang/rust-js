// A `flat_map` of a `HashMap`'s iterators, collected into a `Vec` in a
// crate that reads a capacity: in the build of std's structs, std's
// compiled `FlatMap`, whose hint is std's body's over what it holds,
// grows it as std's does (#1892). Sorted, as a `HashMap`'s order is its
// seed's.
use std::collections::HashMap;

fn main() {
    let mut all: HashMap<i32, HashMap<i32, usize>> = HashMap::new();
    all.entry(1).or_default().insert(2, 3);
    all.entry(1).or_default().insert(4, 5);
    all.entry(6).or_default().insert(7, 8);
    let mut counts: Vec<(i32, i32, usize)> = all
        .iter()
        .flat_map(|(a, next)| next.iter().map(move |(b, n)| (*a, *b, *n)))
        .collect();
    counts.sort();
    println!("{counts:?} {}", counts.capacity());
}
