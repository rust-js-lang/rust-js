//@ native: wasm32
// A stable sort whose comparison can be seen, one that counts or prints,
// is std's driftsort step for step (#925): each comparison std asks, in its
// order, and the order it leaves (`sort_stable_unwinds` panics in it).
// Of lengths 0 to 300, items in random order, of few keys, sorted,
// reversed, in runs and all equal; of a `Copy` type of 8 bytes, one of 24,
// one that isn't `Freeze`, and `String`s, and `sort_by_cached_key`'s
// pairs, whose size is a 32-bit target's (ADR 0090): native Rust here is
// `wasm32-wasip1`'s.
use std::cell::Cell;
use std::cmp::Ordering;

thread_local! {
    static CALLS: Cell<u64> = const { Cell::new(0) };
    static TRACE: Cell<u64> = const { Cell::new(0) };
}

/// A comparison of the items numbered `a` and `b`: counted, and folded
/// into the trace of the sort's calls.
fn seen(a: u32, b: u32) {
    CALLS.set(CALLS.get() + 1);
    let trace = TRACE.get().wrapping_mul(1_000_003) ^ (u64::from(a) << 16 | u64::from(b));
    TRACE.set(trace % 1_000_000_007);
}

/// What a sort did: its calls, their trace, and the items' order after it.
fn report(name: &str, len: usize, ids: impl Iterator<Item = u32>) {
    let order = ids.fold(0u64, |h, id| (h * 31 + u64::from(id)) % 1_000_000_007);
    println!(
        "{name} {len}: {} calls, trace {}, order {order}",
        CALLS.get(),
        TRACE.get()
    );
}

#[derive(Clone, Copy)]
struct Tag {
    key: u32,
    id: u32,
}

#[derive(Clone, Copy)]
struct Wide {
    key: u32,
    id: u32,
    _pad: [u32; 4],
}

/// Not `Freeze`, which std sorts small by insertion alone.
struct Celled {
    key: Cell<u32>,
    id: u32,
}

/// Its own order, which counts each comparison.
#[derive(PartialEq, Eq)]
struct Counted {
    key: u32,
    id: u32,
}

impl PartialOrd for Counted {
    fn partial_cmp(&self, other: &Counted) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Counted {
    fn cmp(&self, other: &Counted) -> Ordering {
        seen(self.id, other.id);
        self.key.cmp(&other.key)
    }
}

/// Its own order, which prints each comparison.
#[derive(PartialEq, Eq)]
struct Shown(u32);

impl PartialOrd for Shown {
    fn partial_cmp(&self, other: &Shown) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Shown {
    fn cmp(&self, other: &Shown) -> Ordering {
        println!("cmp {} {}", self.0, other.0);
        self.0.cmp(&other.0)
    }
}

struct Lcg(u64);

impl Lcg {
    fn next(&mut self) -> u32 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        (self.0 >> 33) as u32
    }
}

const SHAPES: [&str; 6] = ["random", "few", "sorted", "reversed", "runs", "equal"];

fn keys(shape: &str, len: usize) -> Vec<u32> {
    let mut rng = Lcg(len as u64 * 131 + shape.len() as u64);
    (0..len as u32)
        .map(|i| match shape {
            "random" => rng.next() % 1000,
            "few" => rng.next() % 4,
            "sorted" => i,
            "reversed" => len as u32 - i,
            "runs" => (i / 37) * 1000 + rng.next() % 50,
            _ => 7,
        })
        .collect()
}

fn start() {
    CALLS.set(0);
    TRACE.set(0);
}

fn tags(shape: &str, len: usize) {
    let mut v: Vec<Tag> = keys(shape, len)
        .into_iter()
        .zip(0..)
        .map(|(key, id)| Tag { key, id })
        .collect();
    start();
    v.sort_by(|a, b| {
        seen(a.id, b.id);
        a.key.cmp(&b.key)
    });
    report("tags", len, v.iter().map(|t| t.id));
}

fn wide(shape: &str, len: usize) {
    let mut v: Vec<Wide> = (keys(shape, len).into_iter().zip(0..))
        .map(|(key, id)| Wide { key, id, _pad: [0; 4] })
        .collect();
    start();
    v.sort_by_key(|w| {
        seen(w.id, 0);
        w.key
    });
    report("wide", len, v.iter().map(|w| w.id));
}

fn celled(shape: &str, len: usize) {
    let mut v: Vec<Celled> = (keys(shape, len).into_iter().zip(0..))
        .map(|(key, id)| Celled {
            key: Cell::new(key),
            id,
        })
        .collect();
    start();
    v.sort_by(|a, b| {
        seen(a.id, b.id);
        a.key.get().cmp(&b.key.get())
    });
    report("celled", len, v.iter().map(|c| c.id));
}

fn counted(shape: &str, len: usize) {
    let mut v: Vec<Counted> = keys(shape, len)
        .into_iter()
        .zip(0..)
        .map(|(key, id)| Counted { key, id })
        .collect();
    start();
    v.sort();
    report("counted", len, v.iter().map(|c| c.id));
}

/// `sort_by_cached_key` in generic code, whose pairs' type its caller's
/// `K` picks how std sorts.
fn sorted_by_cached<T, K: Ord>(v: &mut [T], key: impl FnMut(&T) -> K) {
    v.sort_by_cached_key(key);
}

fn cached_generic(shape: &str, len: usize) {
    let mut v: Vec<Tag> = keys(shape, len)
        .into_iter()
        .zip(0..)
        .map(|(key, id)| Tag { key, id })
        .collect();
    start();
    sorted_by_cached(&mut v, |t| Counted { key: t.key, id: t.id });
    report("cached generic", len, v.iter().map(|t| t.id));
}

/// A key that's quiet, of an order that counts.
fn keyed(shape: &str, len: usize) {
    let mut v: Vec<Tag> = keys(shape, len)
        .into_iter()
        .zip(0..)
        .map(|(key, id)| Tag { key, id })
        .collect();
    start();
    v.sort_by_key(|t| Counted { key: t.key, id: t.id });
    report("keyed", len, v.iter().map(|t| t.id));
}

/// A comparator of std's order, in reverse, of what holds an order that
/// counts.
fn optional(shape: &str, len: usize) {
    let mut v: Vec<Option<Counted>> = (keys(shape, len).into_iter().zip(0..))
        .map(|(key, id)| (key % 5 != 0).then_some(Counted { key, id }))
        .collect();
    start();
    v.sort_by(|a, b| b.cmp(a));
    report("optional", len, v.iter().map(|c| c.as_ref().map_or(0, |c| c.id + 1)));
}

/// `sort_by_cached_key`, which sorts its `(key, index)` pairs unstably:
/// std's unstable sort's sequence of the keys' comparisons.
fn cached(shape: &str, len: usize) {
    let mut v: Vec<Tag> = keys(shape, len)
        .into_iter()
        .zip(0..)
        .map(|(key, id)| Tag { key, id })
        .collect();
    start();
    v.sort_by_cached_key(|t| Counted { key: t.key, id: t.id });
    report("cached", len, v.iter().map(|t| t.id));
}

fn strings(shape: &str, len: usize) {
    let mut v: Vec<(String, u32)> = (keys(shape, len).into_iter().zip(0..))
        .map(|(key, id)| (format!("{:03}", key % 100), id))
        .collect();
    start();
    v.sort_by(|a, b| {
        seen(a.1, b.1);
        a.0.cmp(&b.0)
    });
    report("strings", len, v.iter().map(|s| s.1));
}

fn main() {
    // Each comparison of seven, as it's asked.
    let mut seven = vec![5, 3, 8, 1, 9, 2, 7];
    seven.sort_by(|a, b| {
        println!("cmp {a} {b}");
        a.cmp(b)
    });
    println!("{seven:?}");
    let mut shown: Vec<Shown> = [5, 3, 8, 1, 9, 2, 7].into_iter().map(Shown).collect();
    shown.sort();

    for len in 0..=300 {
        tags("random", len);
    }
    let lengths = [2, 19, 20, 21, 32, 33, 48, 64, 65, 96, 100, 129, 200, 257, 300];
    for shape in SHAPES {
        println!("-- {shape}");
        for len in lengths {
            tags(shape, len);
            wide(shape, len);
            celled(shape, len);
            counted(shape, len);
            strings(shape, len);
            cached(shape, len);
            cached_generic(shape, len);
            keyed(shape, len);
            optional(shape, len);
        }
    }
}
