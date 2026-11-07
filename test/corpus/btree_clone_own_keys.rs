// A `BTreeMap`'s and a `BTreeSet`'s clone of keys whose `Ord` and `Clone`
// are their own clones each key, then its value, in the keys' order, and
// asks no comparison, as std's `clone_subtree` does; as a clone panics,
// the key cloned for its entry is dropped, then the entries cloned so far
// (#924). Up to eleven keys, one node of std's.
use std::cell::Cell;
use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};
use std::panic::{AssertUnwindSafe, catch_unwind};

thread_local! {
    static BOOM: Cell<u32> = const { Cell::new(0) };
}

/// Ordered by its number past a hundred, so a clone, `n + 100`, is the
/// key it was cloned from.
#[derive(Debug)]
struct D(u32);

fn boom(n: u32, what: &str) {
    if BOOM.get() == n {
        BOOM.set(0);
        panic!("{what} {n}");
    }
}

impl Drop for D {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

impl Clone for D {
    fn clone(&self) -> D {
        println!("clone {}", self.0);
        boom(self.0, "clone");
        D(self.0 + 100)
    }
}

impl PartialEq for D {
    fn eq(&self, other: &D) -> bool {
        self.0 % 100 == other.0 % 100
    }
}

impl Eq for D {}

impl PartialOrd for D {
    fn partial_cmp(&self, other: &D) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for D {
    fn cmp(&self, other: &D) -> Ordering {
        println!("cmp {} {}", self.0, other.0);
        (self.0 % 100).cmp(&(other.0 % 100))
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
    for boom in [0, 1, 3, 12] {
        run("map", boom, || {
            let mut m = BTreeMap::new();
            m.insert(D(3), D(13));
            m.insert(D(1), D(11));
            m.insert(D(2), D(12));
            let w = m.clone();
            println!("{:?}", w);
            println!("{:?}", w.get(&D(2)));
        });
        run("set", boom, || {
            let mut s = BTreeSet::new();
            s.insert(D(3));
            s.insert(D(1));
            s.insert(D(2));
            let t = s.clone();
            println!("{:?}", t);
            println!("{}", t.contains(&D(1)));
        });
    }
    // Of a value with no destructor.
    let mut m = BTreeMap::new();
    m.insert(D(2), 2);
    m.insert(D(1), 1);
    let w = m.clone();
    println!("{:?}", w.keys().collect::<Vec<_>>());
    // In generic code, its class its caller's.
    let w = cloned(&w);
    println!("{:?}", w);
    let mut s = BTreeSet::new();
    s.insert(D(5));
    s.insert(D(4));
    println!("{:?}", cloned_set(&s));
    // Of keys a hashed map finds, in generic code.
    let numbers = BTreeMap::from([(2, "b"), (1, "a")]);
    println!("{:?}", cloned(&numbers));
    println!("{:?}", cloned_set(&BTreeSet::from(["y", "x"])));
}

fn cloned<K: Ord + Clone, V: Clone>(m: &BTreeMap<K, V>) -> BTreeMap<K, V> {
    m.clone()
}

fn cloned_set<T: Ord + Clone>(s: &BTreeSet<T>) -> BTreeSet<T> {
    s.clone()
}
