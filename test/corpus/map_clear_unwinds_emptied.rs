// A map's or a set's `clear()` whose drop of an entry panics leaves it
// empty, each entry dropped once: std's `BTreeMap` takes its entries out
// before it drops them, and its `HashMap` clears the table as a drop
// unwinds; none is dropped again with the map (#899).
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::panic::{AssertUnwindSafe, catch_unwind};

struct D(u32);

impl Drop for D {
    fn drop(&mut self) {
        println!("drop {}", self.0);
        if self.0 == 2 {
            panic!("drop");
        }
    }
}

#[derive(PartialEq, Eq, PartialOrd, Ord)]
struct K(u32);

impl Drop for K {
    fn drop(&mut self) {
        println!("drop key {}", self.0);
        if self.0 == 2 {
            panic!("drop key");
        }
    }
}

fn run(name: &str, f: impl FnOnce()) {
    println!("-- {name}");
    let r = catch_unwind(AssertUnwindSafe(f));
    println!("{}", if r.is_ok() { "ended" } else { "panicked" });
}

fn main() {
    run("BTreeMap", || {
        let mut m = BTreeMap::new();
        m.insert(1, D(1));
        m.insert(2, D(2));
        m.insert(3, D(3));
        m.clear();
        println!("{}", m.len());
    });
    run("BTreeSet", || {
        let mut s = BTreeSet::new();
        s.insert(K(1));
        s.insert(K(2));
        s.insert(K(3));
        s.clear();
        println!("{}", s.len());
    });
    run("HashMap", || {
        let mut m = HashMap::new();
        m.insert(1, D(2));
        m.clear();
        println!("{}", m.len());
    });
    run("cleared, then used", || {
        let mut m = BTreeMap::new();
        m.insert(1, D(1));
        m.insert(2, D(2));
        let r = catch_unwind(AssertUnwindSafe(|| m.clear()));
        println!("{} {}", r.is_ok(), m.len());
        m.insert(4, D(4));
        println!("{}", m.len());
    });
    run("quiet", || {
        let mut m = BTreeMap::new();
        m.insert(1, D(1));
        m.insert(3, D(3));
        m.clear();
        println!("{}", m.len());
    });
}
