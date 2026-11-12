// A generic `collect()` into a `Vec` of a type parameter's items checks
// std's room upfront by the size its caller gives, `TSize`, where its count
// is the program's: a range's, a `repeat_n`'s, a `take(n)` of an endless
// one or of a range, a range's `step_by`, through adapters that keep it, a
// chain or a zip of such, a held one too. So a count past `isize::MAX` bytes panics `capacity overflow` as
// native Rust does, on wasm32 too, and one within it doesn't. A collect of
// a collection's items, of a `filter`'s or into a `Result` takes no `TSize`
// (ADR 0774, amended, #1436).
use std::panic::{self, AssertUnwindSafe};

fn check<R>(what: &str, f: impl FnOnce() -> R) {
    match panic::catch_unwind(AssertUnwindSafe(f)) {
        Ok(_) => println!("{what}: ok"),
        Err(_) => println!("{what}: panicked"),
    }
}

#[derive(Clone)]
struct Loud([u64; 4]);

impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop {}", self.0[0]);
    }
}

fn dropping<T: Clone>(x: T, n: usize) -> usize {
    (1..n).map(|_| x.clone()).collect::<Vec<T>>().len()
}

fn mapped<T: Copy>(x: T, n: usize) -> usize {
    (0..n).map(|_| x).collect::<Vec<T>>().len()
}

fn from_one<T: Copy>(x: T, n: usize) -> usize {
    (1..n).map(|_| x).collect::<Vec<T>>().len()
}

fn reversed<T: Copy>(x: T, n: usize) -> usize {
    (0..n).rev().map(|_| x).collect::<Vec<T>>().len()
}

fn enumerated<T: Copy>(x: T, n: usize) -> usize {
    (0..n).enumerate().map(|_| x).collect::<Vec<T>>().len()
}

fn zipped<T: Copy>(x: T, n: usize) -> usize {
    (0..n).zip(0..n).map(|_| x).collect::<Vec<T>>().len()
}

fn chained<T: Copy>(x: T, n: usize) -> usize {
    (0..n).chain(0..2).map(|_| x).collect::<Vec<T>>().len()
}

fn repeated<T: Copy>(x: T, n: usize) -> usize {
    std::iter::repeat_n(x, n).collect::<Vec<T>>().len()
}

fn taken<T: Copy>(x: T, n: usize) -> usize {
    std::iter::repeat(x).take(n).collect::<Vec<T>>().len()
}

fn range_taken<T: Copy>(x: T, n: usize) -> usize {
    (0..usize::MAX).map(|_| x).take(n).collect::<Vec<T>>().len()
}

fn stepped<T: Copy>(x: T, n: usize) -> usize {
    (0..n).step_by(2).map(|_| x).collect::<Vec<T>>().len()
}

fn inclusive<T: Copy>(x: T, n: usize) -> usize {
    (0..=n).map(|_| x).collect::<Vec<T>>().len()
}

fn held<T: Copy>(x: T, n: usize) -> usize {
    let it = (0..n).map(|_| x);
    it.collect::<Vec<T>>().len()
}

fn of_items<T: Copy>(x: T, items: &[u8]) -> usize {
    items.iter().map(|_| x).collect::<Vec<T>>().len()
}

fn filtered<T: Copy>(x: T, n: usize) -> usize {
    (0..n).filter(|i| i % 2 == 0).map(|_| x).collect::<Vec<T>>().len()
}

fn results<T: Copy>(x: T, n: usize) -> usize {
    (0..n).map(|_| Ok::<T, ()>(x)).collect::<Result<Vec<T>, ()>>().map_or(0, |v| v.len())
}

fn main() {
    let past = usize::MAX / 2;
    let w = [0u64; 4];
    check("map", || mapped(w, past));
    check("map fits", || mapped(w, 3));
    check("dropping", || dropping(Loud([1, 0, 0, 0]), past));
    check("dropping fits", || dropping(Loud([2, 0, 0, 0]), 3));
    check("from one", || from_one(w, past));
    check("from one fits", || from_one(w, 3));
    check("rev", || reversed(w, past));
    check("enumerate", || enumerated(w, past));
    check("zip", || zipped(w, past));
    check("chain", || chained(w, past));
    check("repeat_n", || repeated(w, past));
    check("repeat_n fits", || repeated(w, 3));
    check("repeat take", || taken(w, past));
    check("range take", || range_taken(w, past));
    check("range take fits", || range_taken(w, 3));
    check("step_by", || stepped(w, usize::MAX));
    check("inclusive", || inclusive(w, past));
    check("held", || held(w, past));
    check("held fits", || held(w, 3));
    check("of items", || of_items(w, &[1, 2, 3]));
    check("filter", || filtered(w, 5));
    check("results", || results(w, 3));
}
