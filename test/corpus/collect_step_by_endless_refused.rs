//@ compile-fail: a `step_by(n)` of what has no end
// `collect()` into a `Vec` of a `step_by(n)` of what has no end asks room
// for `usize::MAX / n` items and one more once it gives an item, which
// panics `capacity overflow` or not by `n` and the items' size, which only
// what it holds as it runs says (ADR 0940, amended).
use std::panic::{self, AssertUnwindSafe};

fn main() {
    let numbers = 0..;
    let collected = panic::catch_unwind(AssertUnwindSafe(move || {
        numbers.step_by(2).map(|n: usize| n as u8).collect::<Vec<u8>>()
    }));
    println!("{}", collected.is_err());
}
