// A `HashMap`'s and a `HashSet`'s drop, and `clear()`, drop their entries in
// a plain loop, as hashbrown's `drop_elements` does: an entry whose drop
// panics stops it, the rest leaked, and `clear()` leaves the table empty
// (#920). A `BTreeMap`'s goes on past it, as its `IntoIter`'s guard does.
// Each entry drops alike, so the order a hashed map picks doesn't show.
use std::cell::Cell;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::panic::{AssertUnwindSafe, catch_unwind};

thread_local! {
    static BOOM: Cell<u32> = const { Cell::new(0) };
}

/// Prints as it's dropped, and panics where its number is `BOOM`'s, once.
#[derive(Debug, PartialEq, Eq, Hash)]
struct D(u32);

impl Drop for D {
    fn drop(&mut self) {
        println!("drop {}", self.0);
        if BOOM.get() == self.0 {
            BOOM.set(0);
            panic!("drop {}", self.0);
        }
    }
}

/// A key that prints alike as it's dropped, whatever its number.
#[derive(Debug, PartialEq, Eq, Hash)]
struct K(u32);

impl Drop for K {
    fn drop(&mut self) {
        println!("drop key");
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
    for boom in [0, 1] {
        run("HashMap drop", boom, || {
            let m = HashMap::from([(1, D(1)), (2, D(1)), (3, D(1))]);
            drop(m);
        });
        run("HashMap of keys and values drop", boom, || {
            let m = HashMap::from([(K(5), D(1)), (K(6), D(1))]);
            drop(m);
        });
        run("HashSet drop", boom, || {
            let s = HashSet::from([D(1)]);
            drop(s);
        });
        let mut m = HashMap::from([(1, D(1)), (2, D(1)), (3, D(1))]);
        run("HashMap clear", boom, || m.clear());
        println!("{}", m.len());
        run("BTreeMap drop", boom, || {
            let m = BTreeMap::from([(1, D(1)), (2, D(1)), (3, D(1))]);
            drop(m);
        });
    }
}
