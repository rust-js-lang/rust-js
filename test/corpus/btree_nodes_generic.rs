// A B-tree of a type parameter's keys is its caller's class (ADR 0466):
// of keys whose comparisons can be seen, std's nodes, whose every call asks
// what std's asks, given what they ask besides `cmp` by the caller: the
// keys' `Ord` dictionary, whose `PartialOrd` and `PartialEq` a bulk build
// sorts a set and keeps by, and what its sort picks by the entries' type
// (#926). Each call is traced, `cmp` apart from `partial_cmp` and `==`.
use std::cell::Cell;
use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};

thread_local! {
    static CALLS: Cell<u32> = const { Cell::new(0) };
    static TRACE: Cell<u64> = const { Cell::new(0) };
}

/// A call `kind` of `a` and `b`: counted and traced.
fn seen(kind: u64, a: u32, b: u32) {
    CALLS.set(CALLS.get() + 1);
    let trace = TRACE.get().wrapping_mul(1_000_003) ^ (kind << 40 | u64::from(a) << 20 | u64::from(b));
    TRACE.set(trace % 1_000_000_007);
}

/// A key whose `==` is `cmp`'s `Equal`.
#[derive(Clone, Debug)]
struct Key(u32);

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
        seen(1, self.0, other.0);
        self.0.cmp(&other.0)
    }
}

/// A key whose `==` is derived, which asks nothing, and whose
/// `partial_cmp` is seen apart from its `cmp`.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Derived(u32);

impl PartialOrd for Derived {
    fn partial_cmp(&self, other: &Derived) -> Option<Ordering> {
        seen(2, self.0, other.0);
        Some(self.cmp(other))
    }
}

impl Ord for Derived {
    fn cmp(&self, other: &Derived) -> Ordering {
        seen(3, self.0, other.0);
        self.0.cmp(&other.0)
    }
}

fn counted(name: &str) {
    println!("{name}: {} calls, trace {}", CALLS.get(), TRACE.get());
}

fn distinct<T: Ord>(xs: Vec<T>) -> BTreeSet<T> {
    xs.into_iter().collect()
}

fn tally<K: Ord + Clone>(xs: &[K]) -> BTreeMap<K, u32> {
    let mut seen = BTreeMap::new();
    for x in xs {
        *seen.entry(x.clone()).or_insert(0) += 1;
    }
    seen
}

fn paired<K: Ord, V>(pairs: Vec<(K, V)>) -> BTreeMap<K, V> {
    pairs.into_iter().collect()
}

fn between<K: Ord + Clone, V: Copy>(m: &BTreeMap<K, V>, a: K, b: K) -> Vec<V> {
    m.range(a..b).map(|(_, v)| *v).collect()
}

fn joined<K: Ord + Clone, V: Clone>(m: &BTreeMap<K, V>, mut other: BTreeMap<K, V>, at: &K) -> (BTreeMap<K, V>, BTreeMap<K, V>) {
    let mut copy = m.clone();
    copy.append(&mut other);
    let tail = copy.split_off(at);
    (copy, tail)
}

fn found<K: Ord>(m: &BTreeMap<K, u32>, keys: Vec<K>) -> usize {
    keys.iter().filter(|k| m.contains_key(k)).count()
}

fn main() {
    let keys: Vec<Key> = (0..60).map(|i| Key(i * 7 % 23)).collect();
    let set = distinct(keys.clone());
    counted("distinct");
    let counts = tally(&keys);
    counted("tally");
    let pairs = paired((0..40).map(|i| (Key(i * 11 % 31), i)).collect());
    counted("paired");
    let got = between(&pairs, Key(3), Key(20));
    counted("between");
    let (left, right) = joined(&pairs, paired((0..30).map(|i| (Key(i * 5 % 37), i)).collect()), &Key(15));
    counted("joined");
    println!("{:?}", set.iter().map(|k| k.0).collect::<Vec<_>>());
    println!("{:?}", counts.iter().map(|(k, n)| (k.0, *n)).collect::<Vec<_>>());
    println!("{got:?}");
    println!("{:?} {:?}", left.len(), right.keys().map(|k| k.0).collect::<Vec<_>>());
    // Of a derived `==` and a `partial_cmp` of its own: a set sorted by
    // `<`, and both kept by `!=`, which asks nothing, in generic code and
    // not.
    let derived: Vec<Derived> = (0..50).map(|i| Derived(i * 3 % 17)).collect();
    let set = distinct(derived.clone());
    counted("derived distinct");
    let made: BTreeSet<Derived> = derived.clone().into_iter().collect();
    counted("derived set");
    let map = paired(derived.iter().cloned().zip(0..).collect());
    counted("derived paired");
    let held = found(&map, (0..20).map(Derived).collect());
    counted("derived found");
    println!("{} {} {} {held}", set.len(), made.len(), map.len());
}
