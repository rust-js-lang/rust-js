// A `BTreeMap` of more entries than one node holds, of keys whose
// comparisons can be seen, so std's nodes (#926), cloned where a value's
// clone panics in a later edge's subtree: std's `clone_subtree` drops, as
// the panic unwinds through each node's frame, that frame's key and value
// made before the edge, then what the frame made before them, then on up
// (#1049).
use std::cell::Cell;
use std::cmp::Ordering;
use std::collections::BTreeMap;
use std::panic::{self, AssertUnwindSafe};

thread_local! {
    static CLONES: Cell<u32> = const { Cell::new(0) };
    static COMPARED: Cell<u32> = const { Cell::new(0) };
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

struct Loud(u32);

impl Clone for Loud {
    fn clone(&self) -> Loud {
        let n = CLONES.with(|c| {
            c.set(c.get() + 1);
            c.get()
        });
        if n == 20 {
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
}
