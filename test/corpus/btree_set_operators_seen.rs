// A B-tree set's operators of items whose comparisons can be seen ask what
// std's do, in their order (#961): `&a | &b` is `from_sorted_iter` of
// `self.union(rhs).cloned()`, std's lazy merge asking `cmp`, each item
// cloned as it's pulled, then those `!=` keeps of each two, as its bulk
// build pulls them, pushed into std's nodes, whose later searches ask what
// std's ask. Sets of 0 to 300 items of an order that counts its calls.
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

/// An item of its own order, which counts each comparison, and each clone,
/// as one of itself.
#[derive(Debug)]
struct Key {
    k: u32,
    id: u32,
}

impl Clone for Key {
    fn clone(&self) -> Key {
        seen(self.id, self.id);
        Key { k: self.k, id: self.id }
    }
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

fn report(name: &str, s: &BTreeSet<Key>) {
    let order = s.iter().fold(0u64, |h, k| {
        (h * 31 + u64::from(k.k) * 7 + u64::from(k.id)) % 1_000_000_007
    });
    println!(
        "{name}: {} calls, trace {}, {} items, order {order}",
        CALLS.get(),
        TRACE.get(),
        s.len()
    );
    CALLS.set(0);
    TRACE.set(0);
}

fn main() {
    for n in [0, 3, 40, 300] {
        let span = n * 2 + 1;
        let a = set(n, span);
        let b = set(n / 2, span);
        CALLS.set(0);
        TRACE.set(0);
        let u = &a | &b;
        report("|", &u);
        let i = &a & &b;
        report("&", &i);
        let d = &a - &b;
        report("-", &d);
        let x = &a ^ &b;
        report("^", &x);
        let mut u = u;
        let found = (0..n).filter(|_| u.contains(&key(random(span)))).count();
        u.insert(key(random(span)));
        report(&format!("contains {found}, insert"), &u);
    }
}
