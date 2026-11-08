//@ native: wasm32
// A B-tree of keys whose comparisons can be seen asks what std's nodes ask
// (#926): past eleven keys, std's tree is more than one node, whose shape
// insert's splits, remove's merges and steals, a bulk build, `split_off`
// and `append` leave, and which each later search follows. Maps and sets
// of 0 to 300 keys of an order that counts its comparisons, and of one
// that isn't a total order, which leaves another order in another shape;
// each operation's calls, their trace, and the order it leaves.
use std::cell::Cell;
use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};

thread_local! {
    static CALLS: Cell<u64> = const { Cell::new(0) };
    static TRACE: Cell<u64> = const { Cell::new(0) };
    static SEED: Cell<u32> = const { Cell::new(2_463_534_242) };
    static NEXT: Cell<u32> = const { Cell::new(0) };
}

/// A comparison of the keys numbered `a` and `b`: counted, and folded into
/// the trace of the calls.
fn seen(a: u32, b: u32) {
    CALLS.set(CALLS.get() + 1);
    let trace = TRACE.get().wrapping_mul(1_000_003) ^ (u64::from(a) << 16 | u64::from(b));
    TRACE.set(trace % 1_000_000_007);
}

fn random(below: u32) -> u32 {
    let mut x = SEED.get();
    x ^= x << 13;
    x ^= x >> 17;
    x ^= x << 5;
    SEED.set(x);
    x % below
}

fn next_id() -> u32 {
    NEXT.set(NEXT.get() + 1);
    NEXT.get()
}

/// A key of its own order, which counts each comparison.
#[derive(Clone, Debug)]
struct Key {
    k: u32,
    id: u32,
}

fn key(k: u32) -> Key {
    Key { k, id: next_id() }
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
        seen(self.id, other.id);
        self.k.cmp(&other.k)
    }
}

/// A key of an order that isn't total, which asks nothing it can be seen
/// asking: where the keys are odd and even, the greater is less.
#[derive(Debug)]
struct Odd {
    k: u32,
    id: u32,
}

fn odd(k: u32) -> Odd {
    Odd { k, id: next_id() }
}

impl PartialEq for Odd {
    fn eq(&self, other: &Odd) -> bool {
        self.k == other.k
    }
}

impl Eq for Odd {}

impl PartialOrd for Odd {
    fn partial_cmp(&self, other: &Odd) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Odd {
    fn cmp(&self, other: &Odd) -> Ordering {
        match (self.k + other.k) % 2 {
            0 => self.k.cmp(&other.k),
            _ => other.k.cmp(&self.k),
        }
    }
}

/// What an operation did: its calls, their trace, and the order it left.
fn report(name: &str, n: u32, order: impl Iterator<Item = (u32, u32, u32)>) {
    let order = order.fold(0u64, |h, (k, id, v)| {
        (h * 31 + u64::from(k) * 7 + u64::from(id) * 3 + u64::from(v)) % 1_000_000_007
    });
    println!(
        "{n} {name}: {} calls, trace {}, order {order}",
        CALLS.get(),
        TRACE.get()
    );
}

fn of_map(m: &BTreeMap<Key, u32>) -> impl Iterator<Item = (u32, u32, u32)> + '_ {
    m.iter().map(|(k, v)| (k.k, k.id, *v))
}

fn of_set(s: &BTreeSet<Key>) -> impl Iterator<Item = (u32, u32, u32)> + '_ {
    s.iter().map(|k| (k.k, k.id, 0))
}

fn maps(n: u32) {
    let span = n * 2 + 1;
    let mut m = BTreeMap::new();
    for _ in 0..n {
        m.insert(key(random(span)), random(100));
    }
    report("insert", n, of_map(&m));
    let mut found = 0;
    for _ in 0..n {
        if m.contains_key(&key(random(span))) {
            found += 1;
        }
        if let Some(v) = m.get(&key(random(span))) {
            found += v;
        }
    }
    report(&format!("get {found}"), n, of_map(&m));
    for _ in 0..n / 3 {
        if let Some(v) = m.remove(&key(random(span))) {
            found += v;
        }
    }
    report(&format!("remove {found}"), n, of_map(&m));
    for _ in 0..n {
        *m.entry(key(random(span))).or_insert(0) += 1;
    }
    m.entry(key(random(span))).and_modify(|v| *v += 2).or_insert(7);
    report("entry", n, of_map(&m));
    let mut sum = 0;
    for _ in 0..5 {
        let a = random(span);
        let b = a + random(span - a);
        sum += m.range(key(a)..key(b)).map(|(_, v)| *v).sum::<u32>();
        sum += m.range(key(a)..=key(b)).count() as u32;
        sum += m.range(..key(b)).count() as u32;
    }
    report(&format!("range {sum}"), n, of_map(&m));
    let mut tail = m.split_off(&key(random(span)));
    report("split_off", n, of_map(&m));
    report("split_off's", n, of_map(&tail));
    let mut other: BTreeMap<Key, u32> = (0..n / 2).map(|_| (key(random(span)), random(100))).collect();
    // Searched in the shape a bulk build leaves.
    let held = (0..n / 2).filter(|_| other.contains_key(&key(random(span)))).count();
    report(&format!("collect {held}"), n, of_map(&other));
    // Appended to an empty one, which takes its shape.
    let mut moved = BTreeMap::new();
    moved.append(&mut tail);
    let held = (0..n / 2).filter(|_| moved.contains_key(&key(random(span)))).count();
    report(&format!("append to empty {held}"), n, of_map(&moved));
    let mut tail = moved;
    m.append(&mut tail);
    report("append", n, of_map(&m));
    m.append(&mut other);
    report("append another", n, of_map(&m));
    let ends = [m.pop_first(), m.pop_last()].map(|end| end.map_or(0, |(k, v)| k.k + v));
    let first = m.first_key_value().map_or(0, |(k, _)| k.k);
    report(&format!("ends {ends:?} {first}"), n, of_map(&m));
    m.retain(|k, v| (k.k + *v) % 3 != 0);
    report("retain", n, of_map(&m));
    let mut copy = m.clone();
    for _ in 0..n / 4 {
        copy.insert(key(random(span)), 1);
    }
    report("clone", n, of_map(&copy));
    m.extend((0..n / 4).map(|_| (key(random(span)), 2)));
    report("extend", n, of_map(&m));
    let a = random(span);
    let taken: u32 = m.extract_if(key(a).., |k, _| k.k % 2 == 0).map(|(k, _)| k.k).sum();
    report(&format!("extract_if {taken}"), n, of_map(&m));
    let (lo, hi) = (key(random(span)), key(random(span)));
    let taken: u32 = m.extract_if(..&hi, |k, _| k.k % 3 != 1).map(|(k, _)| k.k).sum();
    let taken = taken + m.extract_if(&lo..=&hi, |_, v| *v % 2 == 0).count() as u32;
    report(&format!("extract_if to an end {taken}"), n, of_map(&m));
    // Each key taken out in turn, the root merged away and popped.
    let mut drained = BTreeMap::new();
    for i in 0..n {
        drained.insert(key(i), i);
    }
    for i in 0..n {
        drained.remove(&key((i * 7) % n.max(1)));
        if i % 5 == 0 {
            drained.insert(key(n + i), i);
        }
    }
    report("drained", n, of_map(&drained));
}

fn sets(n: u32) {
    let span = n * 2 + 1;
    let mut s = BTreeSet::new();
    let mut added = 0;
    for _ in 0..n {
        if s.insert(key(random(span))) {
            added += 1;
        }
    }
    report(&format!("set insert {added}"), n, of_set(&s));
    for _ in 0..n / 3 {
        if s.remove(&key(random(span))) {
            added += 1;
        }
        if let Some(old) = s.replace(key(random(span))) {
            added += old.k;
        }
        if let Some(old) = s.take(&key(random(span))) {
            added += old.k;
        }
    }
    report(&format!("set remove {added}"), n, of_set(&s));
    let a = random(span);
    let in_range = s.range(key(a)..).count();
    let mut tail = s.split_off(&key(random(span)));
    report(&format!("set split_off {in_range}"), n, of_set(&s));
    let mut other: BTreeSet<Key> = (0..n).map(|_| key(random(span))).collect();
    report("set collect", n, of_set(&other));
    s.append(&mut tail);
    s.append(&mut other);
    report("set append", n, of_set(&s));
    s.retain(|k| k.k % 5 != 0);
    let popped = s.pop_first().map_or(0, |k| k.k) + s.pop_last().map_or(0, |k| k.k);
    report(&format!("set retain {popped}"), n, of_set(&s));
}

/// An order that isn't total leaves its keys in the order its nodes' shape
/// does. (Its sort may panic, as std's finds it isn't total: built by
/// `insert`.)
fn odds(n: u32) {
    let span = n * 2 + 1;
    let mut m = BTreeMap::new();
    let mut other = BTreeMap::new();
    for _ in 0..n {
        m.insert(odd(random(span)), random(100));
        other.insert(odd(random(span)), 1);
    }
    for _ in 0..n / 3 {
        m.remove(&odd(random(span)));
    }
    let tail = other.split_off(&odd(random(span)));
    m.append(&mut other);
    let order = m.iter().chain(&tail).fold(0u64, |h, (k, v)| {
        (h * 31 + u64::from(k.k) * 7 + u64::from(k.id) + u64::from(*v)) % 1_000_000_007
    });
    let found = (0..n).filter(|_| m.contains_key(&odd(random(span)))).count();
    println!("{n} odd: {} keys, order {order}, {found} found", m.len());
}

fn main() {
    for n in [0, 1, 5, 11, 12, 13, 17, 23, 40, 64, 100, 150, 233, 300] {
        maps(n);
        sets(n);
        odds(n);
    }
}
