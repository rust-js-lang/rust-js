// A `collect()` into a `Vec` of what the program counts, of items of a type
// known here: a zip's, a chain's, a held map's, a `take(n)`'s of a range,
// checks std's room upfront by its count, as std's `SpecFromIter` reserves a
// `TrustedLen`'s, so a count past `isize::MAX` bytes panics `capacity
// overflow` as native Rust does, on wasm32 too. A count that surely fits,
// literal bounds, is checked not at all, its collect as it reads; and each
// bound is run once, in Rust's order, before any item is made (#1441).
use std::panic::{self, AssertUnwindSafe};

fn check<R>(what: &str, f: impl FnOnce() -> R) {
    match panic::catch_unwind(AssertUnwindSafe(f)) {
        Ok(_) => println!("{what}: ok"),
        Err(_) => println!("{what}: panicked"),
    }
}

fn bound(name: &str, n: usize) -> usize {
    println!("bound {name}");
    n
}

fn main() {
    let past = usize::MAX / 2;
    let w = [0u64; 4];
    check("zip", || (0..past).zip(0..past).map(|_| w).collect::<Vec<_>>().len());
    check("chain", || (0..past).chain(0..2).map(|_| w).collect::<Vec<_>>().len());
    check("range take", || (0..usize::MAX).map(|_| w).take(past).collect::<Vec<_>>().len());
    let it = (0..past).map(|_| w);
    check("held", move || it.collect::<Vec<_>>().len());
    check("zip fits", || (0..3).zip(0..4).map(|_| w).collect::<Vec<_>>().len());
    let both: Vec<usize> = (0..2).chain(5..7).collect();
    println!("{both:?}");
    let pairs: Vec<(usize, usize)> = (0..bound("a", 2)).zip(bound("b", 1)..bound("c", 4)).collect();
    println!("{pairs:?}");
    check("ordered", || {
        (0..bound("d", past))
            .zip(bound("e", 0)..bound("f", past))
            .map(|_| {
                println!("item");
                w
            })
            .collect::<Vec<_>>()
            .len()
    });
}
