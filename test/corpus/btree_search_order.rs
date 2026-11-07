// A `BTreeMap`'s and a `BTreeSet`'s keys of their own `Ord` are compared
// as std's B-tree compares them: the key given against each it holds, in
// order, `cmp(given, held)`, a node's keys from its first; as a comparison
// panics, what the call was given is dropped (#911). Up to eleven keys,
// one node of std's.
use std::cell::Cell;
use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};
use std::panic::{AssertUnwindSafe, catch_unwind};

thread_local! {
    static BOOM: Cell<u32> = const { Cell::new(0) };
}

#[derive(Debug)]
struct D(u32);

impl Drop for D {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

impl PartialEq for D {
    fn eq(&self, other: &D) -> bool {
        self.0 == other.0
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
        let n = BOOM.get();
        if n > 0 {
            BOOM.set(n - 1);
            if n == 1 {
                panic!("cmp");
            }
        }
        self.0.cmp(&other.0)
    }
}

fn map(keys: &[u32]) -> BTreeMap<D, u32> {
    let boom = BOOM.replace(0);
    let mut m = BTreeMap::new();
    for &k in keys {
        m.insert(D(k), k);
    }
    BOOM.set(boom);
    m
}

fn set(keys: &[u32]) -> BTreeSet<D> {
    let boom = BOOM.replace(0);
    let mut s = BTreeSet::new();
    for &k in keys {
        s.insert(D(k));
    }
    BOOM.set(boom);
    s
}

fn run(name: &str, boom: u32, f: impl FnOnce()) {
    println!("-- {name} {boom}");
    BOOM.set(boom);
    let r = catch_unwind(AssertUnwindSafe(f));
    BOOM.set(0);
    println!("{}", if r.is_ok() { "ended" } else { "panicked" });
}

fn main() {
    let keys = [5, 2, 8, 1, 9, 3, 7, 4, 6, 10, 11];
    for boom in 0..4 {
        run("insert", boom, || {
            let mut m = map(&keys[..6]);
            println!("{:?}", m.insert(D(4), 4));
            println!("{:?}", m.insert(D(12), 12));
        });
        run("get", boom, || {
            let m = map(&keys);
            println!("{:?}", m.get(&D(7)));
        });
        run("insert of a value with a drop", boom, || {
            let boom = BOOM.replace(0);
            let mut m = BTreeMap::new();
            m.insert(D(2), D(102));
            m.insert(D(5), D(105));
            BOOM.set(boom);
            println!("{:?}", m.insert(D(3), D(103)));
        });
        run("remove", boom, || {
            let mut m = map(&keys[..5]);
            println!("{:?}", m.remove(&D(2)));
        });
        run("contains_key", boom, || {
            let m = map(&keys[..5]);
            println!("{}", m.contains_key(&D(6)));
        });
        run("split_off", boom, || {
            let mut m = map(&keys[..7]);
            let tail = m.split_off(&D(5));
            println!("{} {}", m.len(), tail.len());
        });
        run("retain", boom, || {
            let mut m = map(&keys[..6]);
            m.retain(|k, x| {
                *x += 1;
                k.0 % 2 == 0
            });
            println!("{:?}", m.values().collect::<Vec<_>>());
            let mut s = set(&keys[..6]);
            s.retain(|k| k.0 > 3);
            println!("{}", s.len());
        });
        run("set insert", boom, || {
            let mut s = set(&keys[..4]);
            println!("{}", s.insert(D(3)));
            println!("{}", s.insert(D(5)));
        });
        run("set contains", boom, || {
            let s = set(&keys[..8]);
            println!("{}", s.contains(&D(4)));
        });
        run("set remove", boom, || {
            let mut s = set(&keys[..3]);
            println!("{}", s.remove(&D(8)));
        });
        run("set take", boom, || {
            let mut s = set(&keys[..3]);
            println!("{:?}", s.take(&D(2)));
        });
        run("set replace", boom, || {
            let mut s = set(&keys[..3]);
            println!("{:?}", s.replace(D(2)));
        });
    }
    // Past one node, of an order that prints nothing: what it finds.
    let mut big: BTreeMap<Q, u32> = BTreeMap::new();
    for k in (0..40).map(|i| (i * 17) % 40) {
        big.insert(Q(k), k * 2);
    }
    println!("{:?} {:?} {}", big.get(&Q(33)), big.get(&Q(41)), big.len());
    big.remove(&Q(7));
    let tail = big.split_off(&Q(30));
    println!("{} {} {:?}", big.len(), tail.len(), tail.keys().next());
}

/// Its own order, which prints nothing: by its number.
#[derive(Debug, PartialEq, Eq)]
struct Q(u32);

impl PartialOrd for Q {
    fn partial_cmp(&self, other: &Q) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Q {
    fn cmp(&self, other: &Q) -> Ordering {
        self.0.cmp(&other.0)
    }
}
