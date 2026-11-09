// A `BTreeMap` of more entries than one node holds, of keys whose
// comparisons can be seen, so std's nodes (#926), cloned where a value's
// clone panics in a later edge's subtree: std's `clone_subtree` drops, as
// the panic unwinds through each node's frame, that frame's key and value
// made before the edge, then what the frame made before them, then on up
// (#1049). Of plain keys too, built in bulk or one by one, and a set's: a
// clone that may panic and drop what it made is std's nodes' to order
// (#1069).
use std::cell::Cell;
use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};
use std::panic::{self, AssertUnwindSafe};

thread_local! {
    static CLONES: Cell<u32> = const { Cell::new(0) };
    static COMPARED: Cell<u32> = const { Cell::new(0) };
    static AT: Cell<u32> = const { Cell::new(20) };
}

/// A key whose comparisons are counted.
#[derive(Clone, PartialEq, Eq)]
struct Key(u32);

impl PartialOrd for Key {
    fn partial_cmp(&self, other: &Key) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Key {
    fn cmp(&self, other: &Key) -> Ordering {
        COMPARED.with(|c| c.set(c.get() + 1));
        self.0.cmp(&other.0)
    }
}

#[derive(PartialEq, Eq, PartialOrd, Ord)]
struct Loud(u32);

impl Clone for Loud {
    fn clone(&self) -> Loud {
        let n = CLONES.with(|c| {
            c.set(c.get() + 1);
            c.get()
        });
        if n == AT.with(|a| a.get()) {
            panic!("clone {}", self.0);
        }
        Loud(self.0)
    }
}

impl Drop for Loud {
    fn drop(&mut self) {
        if CLONES.with(|c| c.get()) > 0 {
            println!("drop {}", self.0);
        }
    }
}

fn main() {
    let mut map = BTreeMap::new();
    for k in 0..30 {
        map.insert(Key(k), Loud(k));
    }
    println!("compared {}", COMPARED.with(|c| c.get()));
    let r = panic::catch_unwind(AssertUnwindSafe(|| map.clone()));
    println!("panicked {}", r.is_err());
    CLONES.with(|c| c.set(0));

    let built: BTreeMap<u32, Loud> = (0..30).map(|k| (k, Loud(k))).collect();
    let r = panic::catch_unwind(AssertUnwindSafe(|| built.clone()));
    println!("built panicked {}", r.is_err());
    CLONES.with(|c| c.set(0));

    let mut inserted = BTreeMap::new();
    for k in (0..40).rev() {
        inserted.insert(k, Loud(k));
    }
    AT.with(|a| a.set(33));
    let r = panic::catch_unwind(AssertUnwindSafe(|| inserted.clone()));
    println!("inserted panicked {}", r.is_err());
    CLONES.with(|c| c.set(0));

    let set: BTreeSet<Loud> = (0..30).map(Loud).collect();
    AT.with(|a| a.set(25));
    let r = panic::catch_unwind(AssertUnwindSafe(|| set.clone()));
    println!("set panicked {}", r.is_err());
    CLONES.with(|c| c.set(0));
}
