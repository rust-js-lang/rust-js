// A map's or a set's `extend` of a `Vec` or an array, whose drop of a
// value it replaces, or of a key it was given, panics, drops what's left
// of what it was given, each entry its key then its value, as std's
// `for_each` drops the `IntoIter` it took as it unwinds (#900).
use std::cell::Cell;
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::panic::{AssertUnwindSafe, catch_unwind};

thread_local! {
    static PANICKED: Cell<bool> = const { Cell::new(false) };
}

/// Panics the first time a run asks.
fn once(what: &str) {
    if !PANICKED.replace(true) {
        panic!("{what}");
    }
}

struct D(u32);

impl Drop for D {
    fn drop(&mut self) {
        println!("drop {}", self.0);
        if self.0 == 2 {
            once("drop");
        }
    }
}

#[derive(PartialEq, Eq, Hash, PartialOrd, Ord)]
struct K(u32);

impl Drop for K {
    fn drop(&mut self) {
        println!("drop key {}", self.0);
        if self.0 == 12 {
            once("drop key");
        }
    }
}

fn run(name: &str, f: impl FnOnce()) {
    println!("-- {name}");
    PANICKED.set(false);
    let r = catch_unwind(AssertUnwindSafe(f));
    println!("{}", if r.is_ok() { "ended" } else { "panicked" });
}

fn main() {
    run("BTreeMap, a Vec", || {
        let mut m = BTreeMap::new();
        m.insert(2, D(2));
        m.extend(vec![(2, D(7)), (6, D(8)), (7, D(9))]);
        println!("{}", m.len());
    });
    run("HashMap, a Vec", || {
        let mut m = HashMap::new();
        m.insert(2, D(2));
        m.extend(vec![(2, D(7)), (6, D(8)), (7, D(9))]);
        println!("{}", m.len());
    });
    run("BTreeMap, an array", || {
        let mut m = BTreeMap::new();
        m.insert(2, D(2));
        m.extend([(2, D(7)), (6, D(8))]);
        println!("{}", m.len());
    });
    run("keys left", || {
        let mut m = HashMap::new();
        m.insert(K(1), D(2));
        m.extend(vec![(K(1), D(5)), (K(6), D(6))]);
        println!("{}", m.len());
    });
    run("BTreeSet", || {
        let mut s = BTreeSet::new();
        s.insert(K(12));
        s.extend(vec![K(12), K(6), K(4)]);
        println!("{}", s.len());
    });
    run("HashSet", || {
        let mut s = HashSet::new();
        s.insert(K(12));
        s.extend(vec![K(12), K(6)]);
        println!("{}", s.len());
    });
    run("none replaced", || {
        let mut m = BTreeMap::new();
        m.insert(1, D(1));
        m.extend(vec![(3, D(3)), (4, D(4))]);
        println!("{}", m.len());
    });
}
