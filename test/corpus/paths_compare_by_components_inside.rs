//@ library-differs
// A path compares, orders and hashes by its components, `a/b` and `a//b/`
// equal, inside what holds it too, as std's derived and structural
// comparisons ask its own: a tuple's `==` and `!=`, an array's, a derived
// struct's `==` and `cmp`, an `Option`'s, a `Vec`'s, a sort, a hash set's and
// a B-tree set's keys, a derived `Hash` map key. A tuple's `==` compared its
// text (#1467). An `OsString`'s is its text's.
use std::collections::{BTreeSet, HashMap, HashSet};
use std::ffi::OsString;
use std::path::PathBuf;

#[derive(PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Clone)]
struct Entry {
    path: PathBuf,
    rank: u8,
}

fn main() {
    let (a, b, c) = (PathBuf::from("a/b"), PathBuf::from("a//b/"), PathBuf::from("a-b"));
    println!(
        "{} {} {}",
        (a.clone(), 1) == (b.clone(), 1),
        (a.clone(), 1) != (b.clone(), 1),
        [a.clone()] == [b.clone()]
    );
    let (x, y) = (
        Entry {
            path: a.clone(),
            rank: 1,
        },
        Entry {
            path: b.clone(),
            rank: 1,
        },
    );
    println!("{} {:?} {:?}", x == y, x.cmp(&y), (a.clone(), 2).cmp(&(c.clone(), 2)));
    println!(
        "{} {}",
        Some(a.clone()) == Some(b.clone()),
        vec![a.clone(), c.clone()] == vec![b.clone(), c.clone()]
    );
    let mut sorted = vec![(c.clone(), 0), (a.clone(), 1), (b.clone(), 0)];
    sorted.sort();
    println!("{sorted:?}");
    let keys: HashSet<(PathBuf, u8)> = [(a.clone(), 1), (b.clone(), 1), (c.clone(), 1)].into_iter().collect();
    let tree: BTreeSet<(PathBuf, u8)> = [(a.clone(), 1), (b.clone(), 1), (c.clone(), 1)].into_iter().collect();
    let entries: HashMap<Entry, &str> = [(x.clone(), "x"), (y.clone(), "y")].into_iter().collect();
    println!(
        "{} {} {:?} {}",
        keys.len(),
        tree.len(),
        tree.iter().map(|(p, _)| p.clone()).collect::<Vec<_>>(),
        entries.len()
    );
    let (o, p) = (OsString::from("a"), OsString::from("a"));
    println!("{} {:?}", (o.clone(), 1) == (p.clone(), 1), (o, 1).cmp(&(p, 2)));
}
