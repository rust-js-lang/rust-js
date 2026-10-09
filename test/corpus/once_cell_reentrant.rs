//@ run-fail: reentrant init
// A `OnceCell` whose init sets it itself panics, as std's does (ADR 0317).

use std::cell::OnceCell;

fn main() {
    let cell: OnceCell<u32> = OnceCell::new();
    let value = cell.get_or_init(|| {
        cell.set(1).unwrap();
        2
    });
    println!("{value}");
}
