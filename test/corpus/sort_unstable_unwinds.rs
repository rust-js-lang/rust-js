// An unstable sort, and `select_nth_unstable`, whose comparison panics
// leaves each item in the slice once, where std's guards put them: its
// insertion sort's gap, its partitions' gaps and its small sort's merge
// each write back as they unwind (#908).
use std::cell::Cell;
use std::cmp::Ordering;
use std::panic::{AssertUnwindSafe, catch_unwind};

thread_local! {
    static BOOM: Cell<u32> = const { Cell::new(0) };
}

fn point() {
    let n = BOOM.get();
    if n > 0 {
        BOOM.set(n - 1);
        if n == 1 {
            panic!("cmp");
        }
    }
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
        point();
        self.0.cmp(&other.0)
    }
}

/// Of more than 96 bytes, which std partitions by Hoare's scheme.
struct Big {
    d: D,
    _pad: [u32; 30],
}

/// Not `Freeze`, which std sorts small by insertion alone.
struct Celled {
    d: D,
    _c: Cell<u8>,
}

fn items(len: u32) -> Vec<D> {
    (0..len).map(|i| D((i * 37 + 11) % 101)).collect()
}

fn shown(v: &[D]) -> String {
    v.iter().map(|d| d.0.to_string()).collect::<Vec<_>>().join(" ")
}

fn sorted(name: &str, len: u32, boom: u32, sort: impl Fn(&mut Vec<D>)) {
    let mut v = items(len);
    BOOM.set(boom);
    let r = catch_unwind(AssertUnwindSafe(|| sort(&mut v)));
    BOOM.set(0);
    println!("{name} {len} {boom} {}: {}", r.is_ok(), shown(&v));
}

fn main() {
    for len in [4, 15, 40, 90] {
        for boom in [1, 2, 3, 7, 20, 45, 90, 160] {
            sorted("sort_unstable", len, boom, |v| v.sort_unstable());
            sorted("sort_unstable_by", len, boom, |v| v.sort_unstable_by(|a, b| b.cmp(a)));
            sorted("sort_unstable_by_key", len, boom, |v| {
                v.sort_unstable_by_key(|d| {
                    point();
                    d.0 % 7
                })
            });
            sorted("select_nth_unstable_by", len, boom, |v| {
                let at = v.len() / 2;
                v.select_nth_unstable_by(at, |a, b| a.cmp(b));
            });
        }
    }
    // Thirty, one small sort: its last comparisons are its merge's.
    for boom in (1..260).step_by(6) {
        sorted("small", 30, boom, |v| v.sort_unstable());
    }
    for boom in [1, 5, 30, 70, 150] {
        let mut v: Vec<Big> = items(60).into_iter().map(|d| Big { d, _pad: [0; 30] }).collect();
        BOOM.set(boom);
        let r = catch_unwind(AssertUnwindSafe(|| v.sort_unstable_by(|a, b| a.d.cmp(&b.d))));
        BOOM.set(0);
        let shown: Vec<String> = v.iter().map(|b| b.d.0.to_string()).collect();
        println!("big {boom} {}: {}", r.is_ok(), shown.join(" "));
        let mut w: Vec<Celled> = items(60).into_iter().map(|d| Celled { d, _c: Cell::new(0) }).collect();
        BOOM.set(boom);
        let r = catch_unwind(AssertUnwindSafe(|| w.sort_unstable_by(|a, b| a.d.cmp(&b.d))));
        BOOM.set(0);
        let shown: Vec<String> = w.iter().map(|c| c.d.0.to_string()).collect();
        println!("celled {boom} {}: {}", r.is_ok(), shown.join(" "));
    }
}
