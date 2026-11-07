// A `BTreeMap`'s `insert` and `extend` of a key it holds an equal one of
// drop the key given first, inside `entry`, then write the value; where
// that drop panics, the value given is dropped as it unwinds and the map
// keeps its old one (#921). A `HashMap`'s, hashbrown's, writes the value,
// then drops the key at its end.
use std::cell::Cell;
use std::collections::{BTreeMap, HashMap};
use std::panic::{AssertUnwindSafe, catch_unwind};

thread_local! {
    static BOOM: Cell<u32> = const { Cell::new(0) };
}

fn boom(n: u32, what: &str) {
    if BOOM.get() == n {
        BOOM.set(0);
        panic!("{what} {n}");
    }
}

/// A value: prints as it's dropped, and panics where its number is
/// `BOOM`'s, once.
#[derive(Debug)]
struct D(u32);

impl Drop for D {
    fn drop(&mut self) {
        println!("drop {}", self.0);
        boom(self.0, "drop");
    }
}

/// A key: prints as it's dropped, and panics where its number and a
/// hundred are `BOOM`'s, once.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct K(u32);

impl Drop for K {
    fn drop(&mut self) {
        println!("drop key {}", self.0);
        boom(self.0 + 100, "drop key");
    }
}

fn run(name: &str, boom: u32, f: impl FnOnce()) {
    println!("-- {name} {boom}");
    BOOM.set(boom);
    let r = catch_unwind(AssertUnwindSafe(f));
    BOOM.set(0);
    println!("{}", if r.is_ok() { "ended" } else { "panicked" });
}

fn main() {
    for boom in [0, 112, 2] {
        run("btree insert", boom, || {
            let mut m = BTreeMap::new();
            m.insert(K(12), D(2));
            println!("{:?}", m.insert(K(12), D(7)));
            println!("{:?}", m);
        });
        run("btree extend", boom, || {
            let mut m = BTreeMap::new();
            m.insert(K(12), D(2));
            m.extend(vec![(K(12), D(7)), (K(13), D(8))]);
            println!("{:?}", m);
        });
        run("hash insert", boom, || {
            let mut m = HashMap::new();
            m.insert(K(12), D(2));
            println!("{:?}", m.insert(K(12), D(7)));
            println!("{}", m.len());
        });
    }
}
