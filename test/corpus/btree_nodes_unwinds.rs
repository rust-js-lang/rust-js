// A B-tree of keys whose comparison may panic, std's nodes (#926): as the
// k-th comparison of an operation panics, what std drops, in its order,
// and the tree it leaves, which later calls search. Of maps of 3 to 80
// keys, each operation that compares, panicking at each of its comparisons
// in turn: `insert`, `entry`, `remove`, `range`, `append`, `extend`,
// `extract_if`, a set's `replace`, and `collect`, whose sort and whose
// deduplication of equal keys panic each. (Not `split_off`'s, which leaves
// std's nodes unfixed, which later calls trip over natively.)
use std::cell::{Cell, RefCell};
use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};
use std::panic::{AssertUnwindSafe, catch_unwind};

thread_local! {
    static CALLS: Cell<u32> = const { Cell::new(0) };
    static PANIC_AT: Cell<u32> = const { Cell::new(0) };
    static NEXT: Cell<u32> = const { Cell::new(0) };
    static LOG: RefCell<String> = const { RefCell::new(String::new()) };
    static SEED: Cell<u32> = const { Cell::new(88_172_645) };
}

fn random(below: u32) -> u32 {
    let mut x = SEED.get();
    x ^= x << 13;
    x ^= x >> 17;
    x ^= x << 5;
    SEED.set(x);
    x % below
}

fn log(what: &str) {
    LOG.with_borrow_mut(|log| {
        log.push_str(what);
        log.push(' ');
    });
}

/// A key whose comparison panics where it's the one `PANIC_AT` counts
/// down to, and whose drop is logged.
struct Key {
    k: u32,
    id: u32,
}

fn key(k: u32) -> Key {
    NEXT.set(NEXT.get() + 1);
    Key { k, id: NEXT.get() }
}

impl Drop for Key {
    fn drop(&mut self) {
        log(&format!("k{}", self.id));
    }
}

impl PartialEq for Key {
    fn eq(&self, other: &Key) -> bool {
        self.cmp(other) == Ordering::Equal
    }
}

impl Eq for Key {}

impl PartialOrd for Key {
    fn partial_cmp(&self, other: &Key) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Key {
    fn cmp(&self, other: &Key) -> Ordering {
        CALLS.set(CALLS.get() + 1);
        let at = PANIC_AT.get();
        if at > 0 {
            PANIC_AT.set(at - 1);
            if at == 1 {
                panic!("cmp {} {}", self.id, other.id);
            }
        }
        self.k.cmp(&other.k)
    }
}

/// A value whose drop is logged.
struct Val(u32);

impl Drop for Val {
    fn drop(&mut self) {
        log(&format!("v{}", self.0));
    }
}

fn map_of(n: u32) -> BTreeMap<Key, Val> {
    let mut m = BTreeMap::new();
    for i in 0..n {
        m.insert(key(random(n * 2)), Val(i));
    }
    LOG.with_borrow_mut(|log| log.clear());
    m
}

fn shown(m: &BTreeMap<Key, Val>) -> String {
    m.iter().map(|(k, v)| format!("{}:{}", k.id, v.0)).collect::<Vec<_>>().join(",")
}

/// `op` of a map of `n` keys, its `k`-th comparison panicking where `k`
/// isn't 0: what it gave, what was dropped, and the map after.
fn run(name: &str, n: u32, k: u32, op: &dyn Fn(&mut BTreeMap<Key, Val>) -> String) -> u32 {
    let seed = SEED.get();
    let mut m = map_of(n);
    CALLS.set(0);
    PANIC_AT.set(k);
    let given = catch_unwind(AssertUnwindSafe(|| op(&mut m))).unwrap_or_else(|_| "panicked".to_string());
    PANIC_AT.set(0);
    let calls = CALLS.get();
    let dropped = LOG.with_borrow_mut(std::mem::take);
    let after = shown(&m);
    // The tree left is searched as std's is.
    CALLS.set(0);
    let found = (0..n * 2).filter(|&i| m.contains_key(&key(i))).count();
    println!("{name} {n} at {k}: {given} | {dropped}| {after} | {found} found, {} calls", CALLS.get());
    drop(m);
    LOG.with_borrow_mut(|log| log.clear());
    if k == 0 {
        SEED.set(seed);
    }
    calls
}

/// `op` run without a panic, then panicking at each of its comparisons,
/// its first `most` and its last ten.
fn each(name: &str, n: u32, most: u32, op: &dyn Fn(&mut BTreeMap<Key, Val>) -> String) {
    let calls = run(name, n, 0, op);
    let last = calls.saturating_sub(10).max(most + 1);
    for k in (1..=calls.min(most)).chain(if most > 0 { last..=calls } else { 1..=0 }) {
        let seed = SEED.get();
        run(name, n, k, op);
        SEED.set(seed);
    }
    SEED.set(SEED.get() ^ 0x9e37);
}

fn main() {
    for n in [3, 12, 30, 80] {
        let span = n * 2;
        each("insert", n, 40, &|m| format!("{:?}", m.insert(key(random(span)), Val(500)).map(|v| v.0)));
        each("entry", n, 40, &|m| {
            let v = m.entry(key(random(span))).or_insert(Val(501));
            v.0 += 1;
            v.0.to_string()
        });
        each("remove", n, 40, &|m| format!("{:?}", m.remove(&key(random(span))).map(|v| v.0)));
        each("range", n, 40, &|m| {
            let a = random(span);
            let (lo, hi) = (key(a), key(a + random(span)));
            m.range(&lo..&hi).map(|(k, _)| k.k).sum::<u32>().to_string()
        });
        each("split_off", n, 0, &|m| {
            let tail = m.split_off(&key(random(span)));
            shown(&tail)
        });
        each("append", n, 40, &|m| {
            let mut other: BTreeMap<Key, Val> = BTreeMap::new();
            PANIC_AT.set(0);
            for i in 0..n / 2 + 1 {
                other.insert(key(random(span)), Val(600 + i));
            }
            m.append(&mut other);
            other.len().to_string()
        });
        each("extend", n, 40, &|m| {
            m.extend((0..3).map(|i| (key(random(span)), Val(700 + i))));
            m.len().to_string()
        });
        each("extract_if", n, 40, &|m| {
            let a = random(span);
            let b = a + random(span);
            let (lo, hi) = (key(a), key(b));
            let taken: Vec<_> = m.extract_if(&lo..&hi, |k, _| k.k % 2 == 0).map(|(k, v)| k.k + v.0).collect();
            format!("{taken:?}")
        });
        each("collect", n, 40, &|m| {
            let made: BTreeMap<Key, Val> = (0..n).map(|i| (key(random(n)), Val(800 + i))).collect();
            let shape = shown(&made);
            drop(made);
            m.len().to_string() + " " + &shape
        });
        each("set", n, 40, &|m| {
            let mut s: BTreeSet<Key> = BTreeSet::new();
            PANIC_AT.set(0);
            for _ in 0..n {
                s.insert(key(random(span)));
            }
            PANIC_AT.set(5);
            let old = s.replace(key(random(span))).map(|k| k.id);
            let collected: BTreeSet<Key> = (0..n).map(|_| key(random(span))).collect();
            format!("{old:?} {} {}", s.len(), collected.len() + m.len())
        });
    }
}
