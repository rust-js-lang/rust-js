// A B-tree set's algebra of items whose comparisons can be seen asks what
// std's lazy iterators ask, in their order (#943): `union` and
// `symmetric_difference` step a merge of the two, asking `cmp` of each pair
// of heads; `intersection` and `difference` stitch the two walks together,
// or search the larger for each of the smaller's items, by the two sets'
// sizes, or answer from their ends; `is_subset`, `is_superset` and
// `is_disjoint` follow. Each step is taken as it's asked, so the calls
// interleave with the consumer's own work. Sets of 0 to 300 items of an
// order that counts its calls, each operation's calls and their trace.
use std::cell::Cell;
use std::cmp::Ordering;
use std::collections::BTreeSet;

thread_local! {
    static CALLS: Cell<u64> = const { Cell::new(0) };
    static TRACE: Cell<u64> = const { Cell::new(0) };
    static SEED: Cell<u32> = const { Cell::new(2_463_534_242) };
    static NEXT: Cell<u32> = const { Cell::new(0) };
}

/// A comparison of the items numbered `a` and `b`: counted, and folded into
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

/// An item of its own order, which counts each comparison.
#[derive(Clone, Debug)]
struct Key {
    k: u32,
    id: u32,
}

fn key(k: u32) -> Key {
    NEXT.set(NEXT.get() + 1);
    Key { k, id: NEXT.get() }
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

fn set(n: u32, span: u32) -> BTreeSet<Key> {
    (0..n).map(|_| key(random(span))).collect()
}

/// Each item an operation gives, as it's given: what it is and the calls
/// asked by then, so a step taken early or late is seen.
fn walked<'a>(name: &str, items: impl Iterator<Item = &'a Key>) {
    CALLS.set(0);
    TRACE.set(0);
    let mut steps = 0u64;
    for item in items {
        steps = (steps * 31 + u64::from(item.k) * 7 + u64::from(item.id) + CALLS.get()) % 1_000_000_007;
    }
    println!("{name}: {} calls, trace {}, steps {steps}", CALLS.get(), TRACE.get());
}

fn asked(name: &str, answer: bool) {
    println!("{name}: {answer}, {} calls, trace {}", CALLS.get(), TRACE.get());
    CALLS.set(0);
    TRACE.set(0);
}

fn pair(name: &str, a: &BTreeSet<Key>, b: &BTreeSet<Key>) {
    walked(&format!("{name} union"), a.union(b));
    walked(&format!("{name} intersection"), a.intersection(b));
    walked(&format!("{name} difference"), a.difference(b));
    walked(&format!("{name} symmetric_difference"), a.symmetric_difference(b));
    // Taken only so far: a lazy one asks no more than it gives.
    walked(&format!("{name} union's first 3"), a.union(b).take(3));
    walked(&format!("{name} intersection's first"), a.intersection(b).take(1));
    CALLS.set(0);
    TRACE.set(0);
    asked(&format!("{name} is_subset"), a.is_subset(b));
    asked(&format!("{name} is_superset"), a.is_superset(b));
    asked(&format!("{name} is_disjoint"), a.is_disjoint(b));
}

fn sets(n: u32) {
    let span = n * 2 + 1;
    let a = set(n, span);
    let b = set(n / 2, span);
    let small = set(n / 20, span);
    // Of `a`'s own items, so a subset, and ends that meet.
    let part: BTreeSet<Key> = a.iter().filter(|k| k.k % 3 != 0).cloned().collect();
    let ends: BTreeSet<Key> = [a.first(), a.last()].into_iter().flatten().cloned().collect();
    let empty = BTreeSet::new();
    println!(
        "{n}: {} {} {} {} {}",
        a.len(),
        b.len(),
        small.len(),
        part.len(),
        ends.len()
    );
    pair(&format!("{n} a b"), &a, &b);
    pair(&format!("{n} b a"), &b, &a);
    pair(&format!("{n} small a"), &small, &a);
    pair(&format!("{n} a small"), &a, &small);
    pair(&format!("{n} part a"), &part, &a);
    pair(&format!("{n} a part"), &a, &part);
    pair(&format!("{n} ends a"), &ends, &a);
    pair(&format!("{n} a empty"), &a, &empty);
}

fn main() {
    for n in [0, 1, 2, 5, 12, 40, 100, 300] {
        sets(n);
    }
}
