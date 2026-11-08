// A stable sort whose comparison panics leaves the items where std's
// driftsort leaves them (#925): its insertion sort's gap, its small sort's
// merge and its runs' merge each write back as they unwind, and a
// partition leaves the slice as it was. Of lengths 4 to 300, in random
// order and in two sorted halves, which merge; a `Copy` type of 8 bytes and
// one that isn't `Freeze`, by `sort_by`, `sort_by_key` and `sort`, each
// sorted whole, then panicking at its 1st, 4th, 30th and 90th comparison,
// its middle one and its last; then the order left, and how many it asked.
use std::cell::Cell;
use std::cmp::Ordering;
use std::panic::{AssertUnwindSafe, catch_unwind};

thread_local! {
    static CALLS: Cell<u64> = const { Cell::new(0) };
    static BOOM: Cell<u64> = const { Cell::new(0) };
}

/// A comparison: counted, and the `BOOM`th panics.
fn compared() {
    let n = CALLS.get() + 1;
    CALLS.set(n);
    if n == BOOM.get() {
        panic!("comparison {n}");
    }
}

#[derive(Clone, Copy)]
struct Tag {
    key: u32,
    id: u32,
}

/// Not `Freeze`, which std sorts small by insertion alone.
struct Celled {
    key: Cell<u32>,
    id: u32,
}

/// Its own order, which panics where `compared` does, and a destructor.
struct Loud {
    key: u32,
    id: u32,
}

impl Drop for Loud {
    fn drop(&mut self) {
        if self.id == 0 {
            println!("drop the first");
        }
    }
}

impl PartialEq for Loud {
    fn eq(&self, other: &Loud) -> bool {
        self.key == other.key
    }
}

impl Eq for Loud {}

impl PartialOrd for Loud {
    fn partial_cmp(&self, other: &Loud) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Loud {
    fn cmp(&self, other: &Loud) -> Ordering {
        compared();
        self.key.cmp(&other.key)
    }
}

/// Keys of `len` items, `distinct` at most, in random order or, `halves`,
/// in two sorted halves.
fn keys(len: usize, distinct: u32, halves: bool) -> Vec<u32> {
    if halves {
        return (0..len as u32)
            .map(|i| (i * 2 + i / (len as u32 / 2)) % (len as u32) % distinct)
            .collect();
    }
    let mut state = len as u64 * 7919 + u64::from(distinct);
    (0..len)
        .map(|_| {
            state = state
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            (state >> 33) as u32 % distinct
        })
        .collect()
}

/// How a sort ended, its calls, and the order of the ids it left.
fn report(name: &str, len: usize, ended: bool, ids: impl Iterator<Item = u32>) {
    let order = ids.fold(0u64, |h, id| (h * 31 + u64::from(id)) % 1_000_000_007);
    let ended = if ended { "ended" } else { "panicked" };
    println!(
        "{name} {len} at {}: {ended} after {} calls, order {order}",
        BOOM.get(),
        CALLS.get()
    );
}

fn start(boom: u64) {
    CALLS.set(0);
    BOOM.set(boom);
}

/// Each sort of `len` items, ended at the `boom`th comparison.
fn sorts(len: usize, halves: bool, boom: u64) -> [u64; 3] {
    let mut tags: Vec<Tag> = (keys(len, 1000, halves).into_iter().zip(0..))
        .map(|(key, id)| Tag { key, id })
        .collect();
    start(boom);
    let ended = catch_unwind(AssertUnwindSafe(|| {
        tags.sort_by(|a, b| {
            compared();
            a.key.cmp(&b.key)
        })
    }));
    report("tags", len, ended.is_ok(), tags.iter().map(|t| t.id));
    let tags_calls = CALLS.get();

    let mut celled: Vec<Celled> = (keys(len, 4, halves).into_iter().zip(0..))
        .map(|(key, id)| Celled {
            key: Cell::new(key),
            id,
        })
        .collect();
    start(boom);
    let ended = catch_unwind(AssertUnwindSafe(|| {
        celled.sort_by_key(|c| {
            compared();
            c.key.get()
        })
    }));
    report("celled", len, ended.is_ok(), celled.iter().map(|c| c.id));
    let celled_calls = CALLS.get();

    let mut loud: Vec<Loud> = (keys(len, 50, halves).into_iter().zip(0..))
        .map(|(key, id)| Loud { key, id })
        .collect();
    start(boom);
    let ended = catch_unwind(AssertUnwindSafe(|| loud.sort()));
    report("loud", len, ended.is_ok(), loud.iter().map(|l| l.id));
    [tags_calls, celled_calls, CALLS.get()]
}

fn main() {
    for halves in [false, true] {
        for len in [4, 21, 40, 70, 150, 300] {
            let whole = sorts(len, halves, 0);
            let most = whole.iter().copied().max().unwrap_or(0);
            for boom in [1, 4, 30, 90, most / 2, most - 1, whole[0] - 1, whole[1] - 1] {
                sorts(len, halves, boom);
            }
        }
    }
}
